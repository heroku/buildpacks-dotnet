use crate::{DotnetBuildpack, DotnetBuildpackError};
use inventory::artifact::Artifact;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, InspectExistingAction, InvalidMetadataAction, LayerContents,
};
use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
use libherokubuildpack::download::download_file;
use libherokubuildpack::log::log_info;
use libherokubuildpack::tar::decompress_tarball;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::env::temp_dir;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub(crate) struct SdkLayerMetadata {
    artifact: Artifact<Version, Sha512>,
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum SdkLayerError {
    #[error("Couldn't create tempfile for .NET SDK: {0}")]
    TempFile(std::io::Error),
    #[error("Couldn't download .NET SDK: {0}")]
    Download(libherokubuildpack::download::DownloadError),
    #[error("Couldn't decompress .NET SDK: {0}")]
    Untar(std::io::Error),
    #[error("Error verifying checksum")]
    ChecksumVerification,
    #[error("Couldn't read tempfile for .NET SDK: {0}")]
    ReadTempFile(std::io::Error),
}

pub(crate) fn handle(
    artifact: &Artifact<Version, Sha512>,
    context: &libcnb::build::BuildContext<DotnetBuildpack>,
) -> Result<(), DotnetBuildpackError> {
    let sdk_layer = context.cached_layer(
        layer_name!("sdk"),
        CachedLayerDefinition {
            build: true,
            launch: true,
            invalid_metadata: &|_| InvalidMetadataAction::DeleteLayer,
            inspect_existing: &|metadata: &SdkLayerMetadata, _path| {
                if &metadata.artifact == artifact {
                    log_info(format!(
                        "Reusing cached .NET SDK version: {}",
                        artifact.version
                    ));
                    InspectExistingAction::Keep
                } else {
                    log_info(format!(
                        "Deleting cached .NET SDK version: {}",
                        metadata.artifact.version
                    ));
                    InspectExistingAction::Delete
                }
            },
        },
    )?;

    if let LayerContents::Empty { .. } = &sdk_layer.contents {
        sdk_layer.replace_metadata(SdkLayerMetadata {
            artifact: artifact.clone(),
        })?;

        libherokubuildpack::log::log_info(format!(
            "Downloading .NET SDK version {} from {}",
            artifact.version, artifact.url
        ));

        let path = temp_dir().as_path().join("dotnetsdk.tar.gz");
        download_file(&artifact.url, path.clone()).map_err(SdkLayerError::Download)?;

        log_info("Verifying checksum");
        let digest = sha512(path.clone()).map_err(SdkLayerError::ReadTempFile)?;
        if artifact.checksum.value != digest {
            Err(SdkLayerError::ChecksumVerification)?;
        }

        log_info(format!(
            "Extracting .NET SDK version: {}",
            &artifact.version
        ));

        log_info(format!("Installing .NET SDK version {}", &artifact.version));
        decompress_tarball(
            &mut File::open(path.clone()).map_err(SdkLayerError::TempFile)?,
            sdk_layer.path(),
        )
        .map_err(SdkLayerError::Untar)?;
        sdk_layer.replace_env(
            &LayerEnv::new()
                .chainable_insert(
                    libcnb::layer_env::Scope::All,
                    ModificationBehavior::Override,
                    "DOTNET_EnableWriteXorExecute",
                    "0",
                )
                .chainable_insert(
                    Scope::All,
                    ModificationBehavior::Prepend,
                    "PATH",
                    format!(
                        "{}:",
                        sdk_layer
                            .path()
                            .as_path()
                            .to_str()
                            .expect("layer to be a str")
                    ),
                ),
        )?;
    };

    Ok(())
}

fn sha512(path: impl AsRef<Path>) -> Result<Vec<u8>, std::io::Error> {
    let mut file = File::open(path.as_ref())?;
    let mut buffer = [0x00; 10 * 1024];
    let mut digest = sha2::Sha512::default();

    let mut read = file.read(&mut buffer)?;
    while read > 0 {
        Digest::update(&mut digest, &buffer[..read]);
        read = file.read(&mut buffer)?;
    }

    Ok(digest.finalize().to_vec())
}

impl From<libcnb::Error<DotnetBuildpackError>> for DotnetBuildpackError {
    fn from(value: libcnb::Error<DotnetBuildpackError>) -> Self {
        value.into()
    }
}
