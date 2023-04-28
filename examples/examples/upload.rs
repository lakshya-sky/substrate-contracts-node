use sp_core::{crypto::Pair, sr25519, Bytes};
use sp_keyring::AccountKeyring;
use subxt::{tx, Config, OnlineClient, PolkadotConfig as DefaultConfig};

type Balance = u128;
type CodeHash = <DefaultConfig as Config>::Hash;
type PairSigner = tx::PairSigner<DefaultConfig, sr25519::Pair>;
type Client = OnlineClient<DefaultConfig>;

mod runtime {
	#[subxt::subxt(runtime_metadata_path = "../metadata.scale")]
	pub mod api {
		#[subxt(substitute_type = "sp_weights::weight_v2::Weight")]
		use ::sp_weights::Weight;
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let signer = PairSigner::new(AccountKeyring::Alice.pair());
	let dest = AccountKeyring::Bob.to_account_id().into();

	let client: Client = OnlineClient::from_url("ws://127.0.0.1:9944").await?;
	println!("{:?}", client.genesis_hash());

	let call = runtime::api::tx().account_abstraction().upload_code(
		code,
		storage_deposit_limit,
		Determinism::Deterministic,
	);

    let t = client.offline().tx().create_unsigned(&call).await?;
	// let call = runtime::api::tx().balances().transfer(dest, 123_456_789_012_345);

	signer.signer().sign(call);

	let hash = client.tx().sign_and_submit_default(&call, &signer).await?;

	println!("Code Upload Extrinsic Submitted: {:?}", hash);

	Ok(())
}
