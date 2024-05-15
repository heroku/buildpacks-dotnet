mod layers;

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

struct DotnetBuildpack;

#[derive(thiserror::Error, Debug)]
enum DotnetBuildpackError {
    #[error(transparent)]
    SdkLayerError(#[from] SdkLayerError),
    #[error("Couldn't parse .NET SDK inventory: {0}")]
    InventoryParse(toml::de::Error),
    #[error("Couldn't parse .NET SDK version: {0}")]
    SemVer(#[from] semver::Error),
    #[error("Couldn't resolve .NET SDK version: {0}")]
    VersionResolution(semver::VersionReq),
}

impl Buildpack for DotnetBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = DotnetBuildpackError;

    fn detect(
        &self,
        _context: libcnb::detect::DetectContext<Self>,
    ) -> libcnb::Result<libcnb::detect::DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    fn build(
        &self,
        context: libcnb::build::BuildContext<Self>,
    ) -> libcnb::Result<libcnb::build::BuildResult, Self::Error> {
        log_header("Resolving .NET SDK version");

        let artifact = resolve_sdk_artifact().map_err(libcnb::Error::BuildpackError)?;
        log_info(format!("Resolved .NET SDK version: {}", artifact.version));

        layers::sdk::handle(&artifact, &context).map_err(libcnb::Error::BuildpackError)?;

        BuildResultBuilder::new().build()
    }
}

const INVENTORY: &str = include_str!("../inventory.toml");

fn resolve_sdk_artifact() -> Result<Artifact<Version, Sha512>, DotnetBuildpackError> {
    let inv: Inventory<Version, Sha512> =
        toml::from_str(INVENTORY).map_err(DotnetBuildpackError::InventoryParse)?;

    let requirement = VersionReq::parse("8.0")?;
    let artifact = match (consts::OS.parse::<Os>(), consts::ARCH.parse::<Arch>()) {
        (Ok(os), Ok(arch)) => inv.resolve(os, arch, &requirement),
        (_, _) => None,
    }
    .ok_or(DotnetBuildpackError::VersionResolution(requirement.clone()))?;

    Ok(artifact.clone())
}

buildpack_main! { DotnetBuildpack }
