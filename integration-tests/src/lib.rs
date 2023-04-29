mod balance;
mod events;
mod extrinsics;
mod upload;

use sp_core::sr25519;
use subxt::{tx, Config, OnlineClient, PolkadotConfig};

pub type DefaultConfig = PolkadotConfig;
pub type Balance = u128;
pub type AccountId = <DefaultConfig as Config>::AccountId;
pub type CodeHash = <DefaultConfig as Config>::Hash;
pub type PairSigner = tx::PairSigner<DefaultConfig, sr25519::Pair>;
pub type Client = OnlineClient<DefaultConfig>;

#[subxt::subxt(runtime_metadata_path = "../metadata.scale")]
mod runtime {}

#[cfg(test)]
mod tests {

	use crate::{balance::TokenMetadata, events::DisplayEvents};

	use super::{extrinsics::*, upload::*, *};
	use contract_transcode::AccountId32;
	use sp_core::H256;
	use sp_keyring::AccountKeyring;
	use std::{path::PathBuf, str::FromStr};

	#[tokio::test]
	async fn test_code_upload() -> Result<(), ErrorVariant> {
		let client: Client = OnlineClient::from_url("ws://127.0.0.1:9944").await?;
		let signer = PairSigner::new(AccountKeyring::Alice.pair());

		// let code = std::fs::read("../../ink_contracts/account_abstraction/Cargo.toml").unwrap();
		// dbg!(code);

		// let dest = AccountKeyring::Bob.to_account_id().into();
		// let tx = runtime::tx().balances().transfer(dest, 123_456_789_012_345);
		// let hash = client.tx().sign_and_submit_default(&tx, &signer).await?;
		let code = upload(
			None,
			Some(&PathBuf::from(
				"../../ink_contracts/account_abstraction/target/ink/account_abstraction.wasm",
			)),
		)?;

		let token_metadata = TokenMetadata::query(&client).await?;

		let bv: Option<balance::BalanceVariant> = None;
		let storage_deposit_limit = bv
			.as_ref()
			.map(|bv| bv.denominate_balance(&token_metadata))
			.transpose()?
			.map(Into::into);

		let call = runtime::tx().account_abstraction().upload_code(
			code.0,
			storage_deposit_limit,
			runtime::runtime_types::pallet_account_abstraction::wasm::Determinism::Deterministic,
		);

		// let hash =
		// 	client.tx().create_signed_with_nonce(&payload, &signer, 0, Default::default())?;
		let hash = client.tx().sign_and_submit_default(&call, &signer).await?;

		println!("Code Upload Extrinsic Submitted: {:?}", hash);

		Ok(())
	}

	#[tokio::test]
	async fn test_account_init() -> Result<(), ErrorVariant> {
		let client: Client = OnlineClient::from_url("ws://127.0.0.1:9944").await?;
		let signer = PairSigner::new(AccountKeyring::Alice.pair());
		// let token_metadata = TokenMetadata::query(&client).await?;
		// let bv: Option<balance::BalanceVariant> = None;
		// let storage_deposit_limit = bv
		// 	.as_ref()
		// 	.map(|bv| bv.denominate_balance(&token_metadata))
		// 	.transpose()?
		// 	.map(Into::into);
		let artifacts = contract_artifacts(
			None,
			Some(&PathBuf::from(
				"../../ink_contracts/account_abstraction/target/ink/account_abstraction.contract",
			)),
		)?;
		let code_hash = H256(artifacts.code_hash()?);
		let transcoder = artifacts.contract_transcoder()?;
		let data = transcoder.encode("new", &["false"])?;
		let salt = vec![0u8; 32];

		let call = runtime::tx().account_abstraction().instantiate(
			0u128,
			runtime::runtime_types::sp_weights::weight_v2::Weight {
				ref_time: 10_000_000_000,
				proof_size: 10_000_000_000,
			},
			None,
			code_hash,
			data,
			salt,
		);

		// let ext = client.tx().create_signed_with_nonce(&call, &signer, 0, Default::default())?;
		// let dry_run_res = ext.dry_run(None).await?;
		// dbg!(dry_run_res);

		let tx_progress = client.tx().sign_and_submit_then_watch_default(&call, &signer).await?;
		let txn_status = tx_progress.wait_for_in_block().await.unwrap_or_else(|err| {
			panic!("error on call `wait_for_in_block`: {err:?}");
		});
		println!("Code Init Extrinsic: {:?}", txn_status);

		Ok(())
	}

	#[tokio::test]
	async fn get_account_pubkey() -> Result<(), ErrorVariant> {
		let client: Client = OnlineClient::from_url("ws://127.0.0.1:9944").await?;
		let signer = PairSigner::new(AccountKeyring::Alice.pair());
		// let token_metadata = TokenMetadata::query(&client).await?;
		// let bv: Option<balance::BalanceVariant> = None;
		// let storage_deposit_limit = bv
		// 	.as_ref()
		// 	.map(|bv| bv.denominate_balance(&token_metadata))
		// 	.transpose()?
		// 	.map(Into::into);
		let artifacts = contract_artifacts(
			None,
			Some(&PathBuf::from(
				"../../ink_contracts/account_abstraction/target/ink/account_abstraction.contract",
			)),
		)?;
		let transcoder = artifacts.contract_transcoder()?;
		let contract =
			AccountId::from_str("5Feb8vZKCbhqpVmzmGH8tzQo4KDWcgGGg1bAsYyxW8MECuQe").unwrap();

		let data = transcoder.encode::<&[&str; 0], _>("get", &[])?;

		let call = runtime::tx().account_abstraction().call(
			contract.into(),
			0u128,
			runtime::runtime_types::sp_weights::weight_v2::Weight {
				ref_time: 10_000_000_000,
				proof_size: 10_000_000_000,
			},
			None,
			data,
		);
		let txn_result = submit_extrinsic(&client, &call, &signer).await?;
		let display_events =
			DisplayEvents::from_events(&txn_result, Some(&transcoder), &client.metadata())?;
		let output = display_events.to_json()?;
		println!("{output}");
		Ok(())
	}
}
