mod balance;
mod extrinsics;
mod upload;

use sp_core::{crypto::Pair, sr25519, Bytes};
use subxt::{tx, Config, OnlineClient, PolkadotConfig as DefaultConfig};

type Balance = u128;
type CodeHash = <DefaultConfig as Config>::Hash;
type PairSigner = tx::PairSigner<DefaultConfig, sr25519::Pair>;
type Client = OnlineClient<DefaultConfig>;

#[cfg(test)]
mod tests {

	use crate::balance::TokenMetadata;

	use super::{extrinsics::*, upload::*, *};
	use sp_keyring::AccountKeyring;
	use std::path::PathBuf;

	#[subxt::subxt(runtime_metadata_path = "../metadata.scale")]
	mod runtime {}

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
		let hash = client.tx().sign_and_submit_default(&call, &signer).await?;

		println!("Code Upload Extrinsic Submitted: {:?}", hash);

		Ok(())
	}
}
