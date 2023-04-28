use std::path::PathBuf;

use crate::extrinsics::{ContractArtifacts, ErrorVariant, WasmCode};

pub fn upload(
	manifest_path: Option<&PathBuf>,
	file: Option<&PathBuf>,
) -> Result<WasmCode, ErrorVariant> {
	let artifacts = ContractArtifacts::from_manifest_or_file(manifest_path, file)?;
	// let artifacts_path = artifacts.artifact_path().to_path_buf();
	Ok(artifacts.code.unwrap())
}
