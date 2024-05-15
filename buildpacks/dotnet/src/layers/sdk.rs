use crate::{DotnetBuildpack, DotnetBuildpackError};
use inventory::artifact::Artifact;
use inventory::checksum::Checksum;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, InspectExistingAction, InvalidMetadataAction, LayerContents, LayerRef,
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

pub(crate) fn handle(
    artifact: &Artifact<Version, Sha512>,
    context: &libcnb::build::BuildContext<DotnetBuildpack>,
) -> Result<LayerRef<DotnetBuildpack, (), ()>, libcnb::Error<DotnetBuildpackError>> {
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
        download_file(&artifact.url, path.clone()).map_err(SdkLayerError::DownloadSdk)?;

        log_info("Verifying checksum");
        verify_checksum(&artifact.checksum, path.clone())?;

        log_info("Installing .NET SDK");
        decompress_tarball(
            &mut File::open(path.clone()).map_err(SdkLayerError::OpenTempFile)?,
            sdk_layer.path(),
        )
        .map_err(SdkLayerError::UntarSdk)?;

        sdk_layer.replace_env(&generate_layer_env(sdk_layer.path().as_path()))?;
    };

    Ok(sdk_layer)
}

fn generate_layer_env(layer_path: &Path) -> LayerEnv {
    LayerEnv::new()
        .chainable_insert(Scope::All, ModificationBehavior::Delimiter, "PATH", ":")
        .chainable_insert(
            Scope::All,
            ModificationBehavior::Prepend,
            "PATH",
            layer_path,
        )
        // Disable .NET tools usage collection: https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-environment-variables#dotnet_cli_telemetry_optout
        .chainable_insert(
            Scope::All,
            ModificationBehavior::Override,
            "DOTNET_CLI_TELEMETRY_OPTOUT",
            "true",
        )
        // Using the buildpack on ARM64 Macs causes failures due to an incompatibility executing on emulated amd64 Docker images (such as builder/heroku:24).
        // This feature is disabled when executing dotnet directly on Apple Silicon (see <https://github.com/dotnet/runtime/pull/70912>).
        // The feature was opt-in for .NET 6.0, but enabled by default in later versions <https://devblogs.microsoft.com/dotnet/announcing-net-6-preview-7/#runtime-wx-write-xor-execute-support-for-all-platforms-and-architectures>.
        // This environment variable disables W^X support.
        // TODO: Investigate performance implications on platforms where this feature is supported.
        .chainable_insert(
            Scope::All,
            ModificationBehavior::Override,
            "DOTNET_EnableWriteXorExecute",
            "0",
        )
        // Mute .NET welcome and telemetry messages: https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-environment-variables#dotnet_nologo
        .chainable_insert(
            Scope::All,
            ModificationBehavior::Override,
            "DOTNET_NOLOGO",
            "true",
        )
        // Specify the location of .NET runtimes as they're not installed in the default location: https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-environment-variables#dotnet_root-dotnet_rootx86-dotnet_root_x86-dotnet_root_x64.
        .chainable_insert(
            Scope::All,
            ModificationBehavior::Override,
            "DOTNET_ROOT",
            layer_path,
        )
        // Enable detection of running in a container: https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-environment-variables#dotnet_running_in_container-and-dotnet_running_in_containers
        // This is used by a few ASP.NET Core workloads.
        // We don't need to set the (now deprecated) `DOTNET_RUNNING_IN_CONTAINER` environment variable as the framework will check for both: https://github.com/dotnet/aspnetcore/blob/8198eeb2b76305677cf94972746c2600d15ff58a/src/DataProtection/DataProtection/src/Internal/ContainerUtils.cs#L86
        .chainable_insert(
            Scope::All,
            ModificationBehavior::Override,
            "DOTNET_RUNNING_IN_CONTAINER",
            "true",
        )
}

fn verify_checksum<D>(checksum: &Checksum<D>, path: impl AsRef<Path>) -> Result<(), SdkLayerError>
where
    D: Digest,
{
    let calculated_checksum = File::open(path.as_ref())
        .map_err(SdkLayerError::OpenTempFile)
        .map(calculate_checksum::<D>)?
        .map_err(SdkLayerError::ReadTempFile)?;

    if calculated_checksum == checksum.value {
        Ok(())
    } else {
        Err(SdkLayerError::VerifyChecksum)
    }
}

fn calculate_checksum<D: Digest>(data: impl Read) -> Result<Vec<u8>, std::io::Error> {
    data.bytes()
        .collect::<Result<Vec<_>, _>>()
        .map(|data| D::digest(data).to_vec())
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum SdkLayerError {
    #[error("Couldn't download .NET SDK: {0}")]
    DownloadSdk(libherokubuildpack::download::DownloadError),
    #[error("Couldn't decompress .NET SDK: {0}")]
    UntarSdk(std::io::Error),
    #[error("Error verifying checksum")]
    VerifyChecksum,
    #[error("Couldn't open tempfile for .NET SDK: {0}")]
    OpenTempFile(std::io::Error),
    #[error("Couldn't read tempfile for .NET SDK: {0}")]
    ReadTempFile(std::io::Error),
}

impl From<SdkLayerError> for libcnb::Error<DotnetBuildpackError> {
    fn from(value: SdkLayerError) -> Self {
        libcnb::Error::BuildpackError(DotnetBuildpackError::SdkLayer(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils;

    #[test]
    fn sdk_layer_env() {
        let layer_env = generate_layer_env(Path::new("/layers/sdk"));

        assert_eq!(
            utils::environment_as_sorted_vector(&layer_env.apply_to_empty(Scope::All)),
            [
                ("DOTNET_CLI_TELEMETRY_OPTOUT", "true"),
                ("DOTNET_EnableWriteXorExecute", "0"),
                ("DOTNET_NOLOGO", "true"),
                ("DOTNET_ROOT", "/layers/sdk"),
                ("DOTNET_RUNNING_IN_CONTAINER", "true"),
                ("PATH", "/layers/sdk")
            ]
        );
    }
}
