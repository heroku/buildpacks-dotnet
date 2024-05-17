mod detect;
mod dotnet_project;
mod layers;
mod tfm;
mod utils;

use crate::dotnet_project::DotnetProject;
use crate::layers::sdk::SdkLayerError;
use crate::tfm::ParseTargetFrameworkError;
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
use std::{fs, io};

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
        log_header(".NET SDK");

        // TODO: Implement and document the project/solution file selection logic
        let project_files = detect::dotnet_project_files(context.app_dir.clone())
            .expect("function to pass after detection");

        let dotnet_project_file = project_files.first().expect("a project file to be present");

        log_info(format!(
            "Detected .NET project file: {}",
            dotnet_project_file.to_string_lossy()
        ));

        let dotnet_project = fs::read_to_string(dotnet_project_file)
            .map_err(DotnetBuildpackError::ReadProjectFile)?
            .parse::<DotnetProject>()
            .map_err(DotnetBuildpackError::ParseDotnetProjectFile)?;

        // TODO: Remove this (currently here for debugging, and making the linter happy)
        log_info(format!(
            "Project type is {:?} using SDK \"{}\" specifies TFM \"{}\"",
            dotnet_project.project_type, dotnet_project.sdk_id, dotnet_project.target_framework
        ));
        let requirement = tfm::parse_target_framework(&dotnet_project.target_framework)
            .map_err(DotnetBuildpackError::ParseTargetFramework)?;

        log_info(format!(
            "Inferred SDK version requirement: {}",
            &requirement.to_string()
        ));
        let artifact = resolve_sdk_artifact(&requirement)?;

        log_info(format!(
            "Resolved .NET SDK version {} ({}-{})",
            artifact.version, artifact.os, artifact.arch
        ));

        let _sdk_layer = layers::sdk::handle(&artifact, &context)?;

        BuildResultBuilder::new().build()
    }
}

const INVENTORY: &str = include_str!("../inventory.toml");

fn resolve_sdk_artifact(
    requirement: &VersionReq,
) -> Result<Artifact<Version, Sha512, Option<()>>, DotnetBuildpackError> {
    let inv: Inventory<Version, Sha512, Option<()>> =
        toml::from_str(INVENTORY).map_err(DotnetBuildpackError::ParseInventory)?;

    let artifact = match (consts::OS.parse::<Os>(), consts::ARCH.parse::<Arch>()) {
        (Ok(os), Ok(arch)) => inv.resolve(os, arch, requirement),
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
    #[error("Error reading project file")]
    ReadProjectFile(io::Error),
    #[error("Error parsing .NET project file")]
    ParseDotnetProjectFile(dotnet_project::ParseError),
    #[error("Error parsing target framework: {0}")]
    ParseTargetFramework(ParseTargetFrameworkError),
}

impl From<DotnetBuildpackError> for libcnb::Error<DotnetBuildpackError> {
    fn from(error: DotnetBuildpackError) -> Self {
        Self::BuildpackError(error)
    }
}

buildpack_main! { DotnetBuildpack }

// The integration tests are imported into the crate so that they can have access to private
// APIs and constants, saving having to (a) run a dual binary/library crate, (b) expose APIs
// publicly for things only used for testing. To prevent the tests from being imported twice,
// automatic integration test discovery is disabled using `autotests = false` in Cargo.toml.
#[cfg(test)]
#[path = "../tests/mod.rs"]
mod tests;
