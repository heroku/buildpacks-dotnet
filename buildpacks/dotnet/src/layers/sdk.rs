use crate::{DotnetBuildpack, DotnetBuildpackError};
use bullet_stream::global::print;
use bullet_stream::style;
use fs_err::File;
use inventory::artifact::Artifact;
use inventory::checksum::Checksum;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, EmptyLayerCause, InvalidMetadataAction, LayerRef, LayerState,
    RestoredLayerAction,
};
use libherokubuildpack::download::DownloadError;
use libherokubuildpack::inventory;
use libherokubuildpack::tar::decompress_tarball;
use retry::delay::Fixed;
use retry::{OperationResult, retry_with_index};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::env::temp_dir;
use std::path::Path;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub(crate) struct SdkLayerMetadata {
    artifact: Artifact<Version, Sha512, Option<()>>,
}

pub(crate) enum CustomCause {
    Ok,
    DifferentSdkArtifact(Artifact<Version, Sha512, Option<()>>),
}

const MAX_RETRIES: usize = 4;
const RETRY_DELAY: Duration = Duration::from_secs(1);

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

            let tarball_path = temp_dir().join("dotnetsdk.tar.gz");

            download_sdk(artifact, &tarball_path).map_err(SdkLayerError::DownloadArchive)?;

            print::sub_bullet("Verifying SDK checksum");
            verify_checksum(&artifact.checksum, &tarball_path)?;

            print::sub_bullet("Installing SDK");
            extract_tarball(&tarball_path, &sdk_layer.path())?;
        }
    }

    Ok(sdk_layer)
}

fn download_sdk(
    artifact: &Artifact<Version, Sha512, Option<()>>,
    path: &Path,
) -> Result<(), DownloadError> {
    let retry_strategy = Fixed::from(RETRY_DELAY).take(MAX_RETRIES);
    retry_with_index(retry_strategy, |attempt_index| {
        // The `retry_with_index` function provides a 1-based `attempt_index` (so the first try is 1).
        let message = if attempt_index == 1 {
            format!("Downloading SDK from {}", style::url(&artifact.url))
        } else {
            format!("Retrying download ({attempt_index}/{})", MAX_RETRIES + 1)
        };
        let log_progress = print::sub_start_timer(message);

        let download_result = download_file(&artifact.url, path);
        match download_result {
            Ok(()) => log_progress.done(),
            Err(ref error) => log_progress.cancel(format!("Failed: {error}")),
        };
        match download_result {
            Ok(()) => OperationResult::Ok(()),
            Err(error @ DownloadError::IoError(_)) => OperationResult::Retry(error),
            Err(error) => OperationResult::Err(error),
        }
    })
    .map_err(|error| error.error)
}

fn download_file(url: &str, destination: &Path) -> Result<(), DownloadError> {
    libherokubuildpack::download::download_file(url, destination)
}

fn verify_checksum<D>(checksum: &Checksum<D>, path: impl AsRef<Path>) -> Result<(), SdkLayerError>
where
    D: Digest,
{
    let calculated_checksum = fs_err::read(path.as_ref())
        .map_err(SdkLayerError::ReadArchive)
        .map(D::digest)?
        .to_vec();

    if calculated_checksum == checksum.value {
        Ok(())
    } else {
        Err(SdkLayerError::VerifyArchiveChecksum {
            expected: checksum.value.clone(),
            actual: calculated_checksum,
        })
    }
}

fn extract_tarball(path: &Path, destination: &Path) -> Result<(), SdkLayerError> {
    decompress_tarball(
        &mut File::open(path).map_err(SdkLayerError::OpenArchive)?.into(),
        destination,
    )
    .map_err(SdkLayerError::DecompressArchive)
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
