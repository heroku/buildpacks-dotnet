use crate::{dotnet_layer_env, DotnetBuildpack, DotnetBuildpackError};
use inventory::artifact::Artifact;
use inventory::checksum::Checksum;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, EmptyLayerCause, InvalidMetadataAction, LayerRef, LayerState,
    RestoredLayerAction,
};
use libcnb::layer_env::Scope;
use libherokubuildpack::download::download_file;
use libherokubuildpack::log::log_info;
use libherokubuildpack::tar::decompress_tarball;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::env::temp_dir;
use std::fs::{self, File};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub(crate) struct SdkLayerMetadata {
    artifact: Artifact<Version, Sha512, Option<()>>,
}

pub(crate) enum CustomCause {
    Ok,
    DifferentSdkArtifact(Artifact<Version, Sha512, Option<()>>),
}

pub(crate) fn handle(
    context: &libcnb::build::BuildContext<DotnetBuildpack>,
    artifact: &Artifact<Version, Sha512, Option<()>>,
) -> Result<LayerRef<DotnetBuildpack, (), CustomCause>, libcnb::Error<DotnetBuildpackError>> {
    let sdk_layer = context.cached_layer(
        layer_name!("sdk"),
        CachedLayerDefinition {
            build: true,
            launch: false,
            invalid_metadata_action: &|_| InvalidMetadataAction::DeleteLayer,
            restored_layer_action: &|metadata: &SdkLayerMetadata, _path| {
                if metadata.artifact == *artifact {
                    (RestoredLayerAction::KeepLayer, CustomCause::Ok)
                } else {
                    (
                        RestoredLayerAction::DeleteLayer,
                        CustomCause::DifferentSdkArtifact(metadata.artifact.clone()),
                    )
                }
            },
        },
    )?;

    match sdk_layer.state {
        LayerState::Restored { .. } => {
            log_info(format!(
                "Reusing cached .NET SDK version: {}",
                artifact.version
            ));
        }
        LayerState::Empty { ref cause } => {
            if let EmptyLayerCause::RestoredLayerAction {
                cause: CustomCause::DifferentSdkArtifact(old_artifact),
            } = cause
            {
                log_info(format!(
                    "Deleting cached .NET SDK version: {}",
                    old_artifact.version
                ));
            }

            sdk_layer.write_metadata(SdkLayerMetadata {
                artifact: artifact.clone(),
            })?;

            libherokubuildpack::log::log_info(format!(
                "Downloading .NET SDK version {} from {}",
                artifact.version, artifact.url
            ));

            let path = temp_dir().as_path().join("dotnetsdk.tar.gz");
            download_file(&artifact.url, path.clone()).map_err(SdkLayerError::DownloadSdk)?;

            log_info("Verifying checksum");
            verify_checksum(&artifact.checksum, path.clone())?;

            log_info("Installing .NET SDK");
            decompress_tarball(
                &mut File::open(path.clone()).map_err(SdkLayerError::OpenSdkArchive)?,
                sdk_layer.path(),
            )
            .map_err(SdkLayerError::UntarSdk)?;

            sdk_layer.write_env(dotnet_layer_env::generate_layer_env(
                sdk_layer.path().as_path(),
                &Scope::Build,
            ))?;
        }
    }

    Ok(sdk_layer)
}

fn verify_checksum<D>(checksum: &Checksum<D>, path: impl AsRef<Path>) -> Result<(), SdkLayerError>
where
    D: Digest,
{
    let calculated_checksum = fs::read(path.as_ref())
        .map(|data| D::digest(data).to_vec())
        .map_err(SdkLayerError::ReadSdkArchive)?;

    if calculated_checksum == checksum.value {
        Ok(())
    } else {
        Err(SdkLayerError::VerifyChecksum {
            expected: checksum.value.clone(),
            actual: calculated_checksum,
        })
    }
}

#[derive(Debug)]
pub(crate) enum SdkLayerError {
    DownloadSdk(libherokubuildpack::download::DownloadError),
    UntarSdk(std::io::Error),
    VerifyChecksum { expected: Vec<u8>, actual: Vec<u8> },
    OpenSdkArchive(std::io::Error),
    ReadSdkArchive(std::io::Error),
}

impl From<SdkLayerError> for libcnb::Error<DotnetBuildpackError> {
    fn from(value: SdkLayerError) -> Self {
        libcnb::Error::BuildpackError(DotnetBuildpackError::SdkLayer(value))
    }
}
