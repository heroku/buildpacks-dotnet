use crate::{dotnet_layer_env, DotnetBuildpack, DotnetBuildpackError};
use buildpacks_jvm_shared::output::{self, BuildpackOutputTextSection};
use inventory::artifact::Artifact;
use inventory::checksum::Checksum;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, EmptyLayerCause, InvalidMetadataAction, LayerRef, LayerState,
    RestoredLayerAction,
};
use libcnb::layer_env::Scope;
use libherokubuildpack::download::{download_file, DownloadError};
use libherokubuildpack::inventory;
use libherokubuildpack::tar::decompress_tarball;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::env::temp_dir;
use std::fs::{self, File};
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub(crate) struct SdkLayerMetadata {
    artifact: Artifact<Version, Sha512, Option<()>>,
}

pub(crate) enum CustomCause {
    Ok,
    DifferentSdkArtifact(Artifact<Version, Sha512, Option<()>>),
}

const MAX_RETRIES: u8 = 4;

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

    output::print_section("SDK installation");

    match sdk_layer.state {
        LayerState::Restored { .. } => {
            output::print_subsection(format!("Reusing cached SDK (version {})", artifact.version));
        }
        LayerState::Empty { ref cause } => {
            if let EmptyLayerCause::RestoredLayerAction {
                cause: CustomCause::DifferentSdkArtifact(old_artifact),
            } = cause
            {
                output::print_subsection(format!(
                    "Deleting cached .NET SDK (version {})",
                    old_artifact.version
                ));
            }

            sdk_layer.write_metadata(SdkLayerMetadata {
                artifact: artifact.clone(),
            })?;

            let tarball_path = temp_dir().join("dotnetsdk.tar.gz");
            let mut download_attempts = 0;
            output::track_timing(|| {
                while download_attempts <= MAX_RETRIES {
                    output::print_subsection(vec![
                        BuildpackOutputTextSection::regular("Downloading SDK from "),
                        BuildpackOutputTextSection::Url(artifact.clone().url),
                    ]);
                    match download_file(&artifact.url, &tarball_path) {
                        Err(DownloadError::IoError(error)) if download_attempts < MAX_RETRIES => {
                            output::print_subsection(format!("Error: {error}"));
                        }
                        result => {
                            return result.map_err(SdkLayerError::DownloadArchive);
                        }
                    }
                    download_attempts += 1;
                    thread::sleep(Duration::from_secs(1));
                    output::print_subsection("Retrying...");
                }
                Ok(())
            })?;

            output::print_subsection("Verifying SDK checksum");
            verify_checksum(&artifact.checksum, &tarball_path)?;

            output::print_subsection("Installing SDK");
            decompress_tarball(
                &mut File::open(&tarball_path).map_err(SdkLayerError::OpenArchive)?,
                sdk_layer.path(),
            )
            .map_err(SdkLayerError::DecompressArchive)?;

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
        .map_err(SdkLayerError::ReadArchive)?;

    if calculated_checksum == checksum.value {
        Ok(())
    } else {
        Err(SdkLayerError::VerifyArchiveChecksum {
            expected: checksum.value.clone(),
            actual: calculated_checksum,
        })
    }
}

#[derive(Debug)]
pub(crate) enum SdkLayerError {
    DownloadArchive(libherokubuildpack::download::DownloadError),
    DecompressArchive(std::io::Error),
    VerifyArchiveChecksum { expected: Vec<u8>, actual: Vec<u8> },
    OpenArchive(std::io::Error),
    ReadArchive(std::io::Error),
}

impl From<SdkLayerError> for libcnb::Error<DotnetBuildpackError> {
    fn from(value: SdkLayerError) -> Self {
        libcnb::Error::BuildpackError(DotnetBuildpackError::SdkLayer(value))
    }
}
