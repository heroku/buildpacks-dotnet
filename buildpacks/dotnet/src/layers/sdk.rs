use crate::{dotnet_layer_env, DotnetBuildpack, DotnetBuildpackError};
use bullet_stream::{state, style, Print};
use inventory::artifact::Artifact;
use inventory::checksum::Checksum;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, EmptyLayerCause, InvalidMetadataAction, LayerRef, LayerState,
    RestoredLayerAction,
};
use libcnb::layer_env::Scope;
use libherokubuildpack::download::download_file;
use libherokubuildpack::inventory;
use libherokubuildpack::tar::decompress_tarball;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::env::temp_dir;
use std::fs::{self, File};
use std::io::Stdout;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub(crate) struct SdkLayerMetadata {
    artifact: Artifact<Version, Sha512, Option<()>>,
}

pub(crate) enum CustomCause {
    Ok,
    DifferentSdkArtifact(Artifact<Version, Sha512, Option<()>>),
}

type HandleResult = Result<
    (
        LayerRef<DotnetBuildpack, (), CustomCause>,
        Print<state::Bullet<Stdout>>,
    ),
    libcnb::Error<DotnetBuildpackError>,
>;

pub(crate) fn handle(
    context: &libcnb::build::BuildContext<DotnetBuildpack>,
    log: Print<state::Bullet<Stdout>>,
    artifact: &Artifact<Version, Sha512, Option<()>>,
) -> HandleResult {
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

    let mut log_bullet = log.bullet("SDK installation");

    match sdk_layer.state {
        LayerState::Restored { .. } => {
            log_bullet =
                log_bullet.sub_bullet(format!("Reusing cached SDK (version {})", artifact.version));
        }
        LayerState::Empty { ref cause } => {
            if let EmptyLayerCause::RestoredLayerAction {
                cause: CustomCause::DifferentSdkArtifact(old_artifact),
            } = cause
            {
                log_bullet = log_bullet.sub_bullet(format!(
                    "Deleting cached .NET SDK (version {})",
                    old_artifact.version
                ));
            }

            sdk_layer.write_metadata(SdkLayerMetadata {
                artifact: artifact.clone(),
            })?;

            let log_background_bullet = log_bullet.start_timer(format!(
                "Downloading SDK from {}",
                style::url(artifact.clone().url)
            ));

            let path = temp_dir().as_path().join("dotnetsdk.tar.gz");
            download_file(&artifact.url, path.clone()).map_err(SdkLayerError::DownloadArchive)?;
            log_bullet = log_background_bullet.done();

            log_bullet = log_bullet.sub_bullet("Verifying SDK checksum");
            verify_checksum(&artifact.checksum, path.clone())?;

            log_bullet = log_bullet.sub_bullet("Installing SDK");
            decompress_tarball(
                &mut File::open(path.clone()).map_err(SdkLayerError::OpenArchive)?,
                sdk_layer.path(),
            )
            .map_err(SdkLayerError::DecompressArchive)?;

            sdk_layer.write_env(dotnet_layer_env::generate_layer_env(
                sdk_layer.path().as_path(),
                &Scope::Build,
            ))?;
        }
    }

    Ok((sdk_layer, log_bullet.done()))
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
