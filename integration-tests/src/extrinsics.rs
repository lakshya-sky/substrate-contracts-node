use anyhow::{anyhow, Context, Ok, Result};
use contract_build::{name_value_println, CrateMetadata, Verbosity, VerbosityFlags};
use contract_metadata::ContractMetadata;
use contract_transcode::ContractMessageTranscoder;
use sp_runtime::DispatchError;
use std::{
	fmt::{self, Debug, Display},
	path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct ContractArtifacts {
	/// The original artifact path
	artifacts_path: PathBuf,
	/// The expected path of the file containing the contract metadata.
	metadata_path: PathBuf,
	/// The deserialized contract metadata if the expected metadata file exists.
	metadata: Option<ContractMetadata>,
	/// The Wasm code of the contract if available.
	pub code: Option<WasmCode>,
}

impl ContractArtifacts {
	/// Load contract artifacts.
	pub fn from_manifest_or_file(
		manifest_path: Option<&PathBuf>,
		file: Option<&PathBuf>,
	) -> Result<ContractArtifacts> {
		let artifact_path = match (manifest_path, file) {
			(manifest_path, None) => {
				let crate_metadata = CrateMetadata::from_manifest_path(manifest_path)?;

				if crate_metadata.contract_bundle_path().exists() {
					crate_metadata.contract_bundle_path()
				} else if crate_metadata.metadata_path().exists() {
					crate_metadata.metadata_path()
				} else {
					anyhow::bail!(
						"Failed to find any contract artifacts in target directory. \n\
                        Run `cargo contract build --release` to generate the artifacts."
					)
				}
			},
			(None, Some(artifact_file)) => artifact_file.clone(),
			(Some(_), Some(_)) => {
				anyhow::bail!("conflicting options: --manifest-path and --file")
			},
		};
		Self::from_artifact_path(artifact_path.as_path())
	}
	/// Given a contract artifact path, load the contract code and metadata where possible.
	fn from_artifact_path(path: &Path) -> Result<Self> {
		tracing::debug!("Loading contracts artifacts from `{}`", path.display());
		let (metadata_path, metadata, code) = match path.extension().and_then(|ext| ext.to_str()) {
			Some("contract") | Some("json") => {
				let metadata = ContractMetadata::load(path)?;
				let code = metadata.clone().source.wasm.map(|wasm| WasmCode(wasm.0));
				(PathBuf::from(path), Some(metadata), code)
			},
			Some("wasm") => {
				let file_name = path
					.file_stem()
					.context("WASM bundle file has unreadable name")?
					.to_str()
					.context("Error parsing filename string")?;
				let code = Some(WasmCode(std::fs::read(path)?));
				let dir = path.parent().map_or_else(PathBuf::new, PathBuf::from);
				let metadata_path = dir.join(format!("{file_name}.json"));
				if !metadata_path.exists() {
					(metadata_path, None, code)
				} else {
					let metadata = ContractMetadata::load(&metadata_path)?;
					(metadata_path, Some(metadata), code)
				}
			},
			Some(ext) => anyhow::bail!(
				"Invalid artifact extension {ext}, expected `.contract`, `.json` or `.wasm`"
			),
			None => {
				anyhow::bail!(
					"Artifact path has no extension, expected `.contract`, `.json`, or `.wasm`"
				)
			},
		};
		Ok(Self { artifacts_path: path.into(), metadata_path, metadata, code })
	}

	/// Get the path of the artifact file used to load the artifacts.
	pub fn artifact_path(&self) -> &Path {
		self.artifacts_path.as_path()
	}

	/// Get contract metadata, if available.
	///
	/// ## Errors
	/// - No contract metadata could be found.
	/// - Invalid contract metadata.
	pub fn metadata(&self) -> Result<ContractMetadata> {
		self.metadata.clone().ok_or_else(|| {
			anyhow!(
				"No contract metadata found. Expected file {}",
				self.metadata_path.as_path().display()
			)
		})
	}

	/// Get the code hash from the contract metadata.
	pub fn code_hash(&self) -> Result<[u8; 32]> {
		let metadata = self.metadata()?;
		Ok(metadata.source.hash.0)
	}

	/// Construct a [`ContractMessageTranscoder`] from contract metadata.
	pub fn contract_transcoder(&self) -> Result<ContractMessageTranscoder> {
		let metadata = self.metadata()?;
		ContractMessageTranscoder::try_from(metadata)
			.context("Failed to deserialize ink project metadata from contract metadata")
	}
}

/// The Wasm code of a contract.
#[derive(Debug)]
pub struct WasmCode(pub Vec<u8>);

impl WasmCode {
	/// The hash of the contract code: uniquely identifies the contract code on-chain.
	pub fn code_hash(&self) -> [u8; 32] {
		contract_build::code_hash(&self.0)
	}
}
#[derive(serde::Serialize)]
pub enum ErrorVariant {
	#[serde(rename = "module_error")]
	Module(ModuleError),
	#[serde(rename = "generic_error")]
	Generic(GenericError),
}

impl From<subxt::Error> for ErrorVariant {
	fn from(error: subxt::Error) -> Self {
		match error {
			subxt::Error::Runtime(subxt::error::DispatchError::Module(module_err)) => module_err
				.details()
				.map(|details| {
					ErrorVariant::Module(ModuleError {
						pallet: details.pallet().to_string(),
						error: details.error().to_string(),
						docs: details.docs().to_vec(),
					})
				})
				.unwrap_or_else(|err| {
					ErrorVariant::Generic(GenericError::from_message(format!(
						"Error extracting subxt error details: {}",
						err
					)))
				}),
			err => ErrorVariant::Generic(GenericError::from_message(err.to_string())),
		}
	}
}

impl From<anyhow::Error> for ErrorVariant {
	fn from(error: anyhow::Error) -> Self {
		Self::Generic(GenericError::from_message(format!("{error:?}")))
	}
}

impl From<&str> for ErrorVariant {
	fn from(err: &str) -> Self {
		Self::Generic(GenericError::from_message(err.to_owned()))
	}
}

#[derive(serde::Serialize)]
pub struct ModuleError {
	pub pallet: String,
	pub error: String,
	pub docs: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct GenericError {
	error: String,
}

impl GenericError {
	pub fn from_message(error: String) -> Self {
		GenericError { error }
	}
}

impl ErrorVariant {
	pub fn from_dispatch_error(
		error: &DispatchError,
		metadata: &subxt::Metadata,
	) -> anyhow::Result<ErrorVariant> {
		match error {
			DispatchError::Module(err) => {
				let details = metadata.error(err.index, err.error[0])?;
				Ok(ErrorVariant::Module(ModuleError {
					pallet: details.pallet().to_owned(),
					error: details.error().to_owned(),
					docs: details.docs().to_owned(),
				}))
			},
			err => Ok(ErrorVariant::Generic(GenericError::from_message(format!(
				"DispatchError: {err:?}"
			)))),
		}
	}
}

impl Debug for ErrorVariant {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		<Self as Display>::fmt(self, f)
	}
}

impl Display for ErrorVariant {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			ErrorVariant::Module(err) => f.write_fmt(format_args!(
				"ModuleError: {}::{}: {:?}",
				err.pallet, err.error, err.docs
			)),
			ErrorVariant::Generic(err) => write!(f, "{}", err.error),
		}
	}
}
