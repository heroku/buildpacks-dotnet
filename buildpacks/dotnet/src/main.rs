mod detect;
mod layers;
mod utils;

use crate::layers::sdk::SdkLayerError;
use inventory::artifact::{Arch, Artifact, Os};
use inventory::inventory::Inventory;
use libcnb::build::BuildResultBuilder;
use libcnb::detect::DetectResultBuilder;
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::{buildpack_main, Buildpack};
use libherokubuildpack::log::{log_header, log_info};
use semver::{Version, VersionReq};
use sha2::Sha512;
use std::env::consts;
use std::io;

struct DotnetBuildpack;

impl Buildpack for DotnetBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = DotnetBuildpackError;

    fn detect(
        &self,
        context: libcnb::detect::DetectContext<Self>,
    ) -> libcnb::Result<libcnb::detect::DetectResult, Self::Error> {
        if detect::dotnet_project_files(context.app_dir)
            .map_err(DotnetBuildpackError::BuildpackDetection)?
            .is_empty()
        {
            log_info("No .NET project files (such as `foo.csproj`) found.");
            DetectResultBuilder::fail().build()
        } else {
            DetectResultBuilder::pass().build()
        }
    }

    fn build(
        &self,
        context: libcnb::build::BuildContext<Self>,
    ) -> libcnb::Result<libcnb::build::BuildResult, Self::Error> {
        log_header("Determining .NET SDK version");

        let artifact = resolve_sdk_artifact().map_err(libcnb::Error::BuildpackError)?;

        log_info(format!(
            "Using .NET SDK version {} ({}-{})",
            artifact.version, artifact.os, artifact.arch
        ));

        let _sdk_layer = layers::sdk::handle(&artifact, &context)?;

        BuildResultBuilder::new().build()
    }
}

const INVENTORY: &str = include_str!("../inventory.toml");

fn resolve_sdk_artifact() -> Result<Artifact<Version, Sha512, ()>, DotnetBuildpackError> {
    let inv: Inventory<Version, Sha512, ()> =
        toml::from_str(INVENTORY).map_err(DotnetBuildpackError::ParseInventory)?;

    let requirement = VersionReq::parse("8.0")?;
    let artifact = match (consts::OS.parse::<Os>(), consts::ARCH.parse::<Arch>()) {
        (Ok(os), Ok(arch)) => inv.resolve(os, arch, &requirement),
        (_, _) => None,
    }
    .ok_or(DotnetBuildpackError::ResolveSdkVersion(requirement.clone()))?;

    Ok(artifact.clone())
}

#[derive(thiserror::Error, Debug)]
enum DotnetBuildpackError {
    #[error("Error when performing buildpack detection")]
    BuildpackDetection(io::Error),
    #[error(transparent)]
    SdkLayer(#[from] SdkLayerError),
    #[error("Couldn't parse .NET SDK inventory: {0}")]
    ParseInventory(toml::de::Error),
    #[error("Couldn't parse .NET SDK version: {0}")]
    ParseSdkVersion(#[from] semver::Error),
    #[error("Couldn't resolve .NET SDK version: {0}")]
    ResolveSdkVersion(semver::VersionReq),
}

impl From<DotnetBuildpackError> for libcnb::Error<DotnetBuildpackError> {
    fn from(error: DotnetBuildpackError) -> Self {
        Self::BuildpackError(error)
    }
}

buildpack_main! { DotnetBuildpack }
