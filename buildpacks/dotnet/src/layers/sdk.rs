use crate::{DotnetBuildpack, DotnetBuildpackError};
use bullet_stream::global::print;
use bullet_stream::style;
use inventory::artifact::Artifact;
use inventory::checksum::Checksum;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, EmptyLayerCause, InvalidMetadataAction, LayerRef, LayerState,
    RestoredLayerAction,
};
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
    available_at_launch: bool,
    artifact: &Artifact<Version, Sha512, Option<()>>,
) -> Result<LayerRef<DotnetBuildpack, (), CustomCause>, libcnb::Error<DotnetBuildpackError>> {
    let sdk_layer = context.cached_layer(
        layer_name!("sdk"),
        CachedLayerDefinition {
            build: true,
            launch: available_at_launch,
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

    print::bullet("SDK installation");

    match sdk_layer.state {
        LayerState::Restored { .. } => {
            print::sub_bullet(format!("Reusing cached SDK (version {})", artifact.version));
        }
        LayerState::Empty { ref cause } => {
            if let EmptyLayerCause::RestoredLayerAction {
                cause: CustomCause::DifferentSdkArtifact(old_artifact),
            } = cause
            {
                print::sub_bullet(format!(
                    "Deleting cached .NET SDK (version {})",
                    old_artifact.version
                ));
            }

            sdk_layer.write_metadata(SdkLayerMetadata {
                artifact: artifact.clone(),
            })?;

            let mut log_background_bullet = print::sub_start_timer(format!(
                "Downloading SDK from {}",
                style::url(artifact.clone().url)
            ));

            let tarball_path = temp_dir().join("dotnetsdk.tar.gz");
            let mut download_attempts = 0;
            while download_attempts <= MAX_RETRIES {
                match download_file(&artifact.url, &tarball_path) {
                    Err(DownloadError::IoError(error)) if download_attempts < MAX_RETRIES => {
                        let sub_bullet = log_background_bullet.cancel(format!("{error}"));
                        download_attempts += 1;
                        thread::sleep(Duration::from_secs(1));
                        log_background_bullet = sub_bullet.start_timer("Retrying");
                    }
                    result => {
                        result.map_err(SdkLayerError::DownloadArchive)?;
                        let _ = log_background_bullet.done();
                        break;
                    }
                }
            }

            print::sub_bullet("Verifying SDK checksum");
            verify_checksum(&artifact.checksum, &tarball_path)?;

            print::sub_bullet("Installing SDK");
            decompress_tarball(
                &mut File::open(&tarball_path).map_err(SdkLayerError::OpenArchive)?,
                sdk_layer.path(),
            )
            .map_err(SdkLayerError::DecompressArchive)?;
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
