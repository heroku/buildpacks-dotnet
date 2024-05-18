mod detect;
mod dotnet_project;
mod global_json;
mod layers;
mod tfm;
mod utils;

use crate::global_json::GlobalJsonError;
use crate::layers::sdk::SdkLayerError;
use crate::tfm::ParseTargetFrameworkError;
use libcnb::build::BuildResultBuilder;
use libcnb::detect::DetectResultBuilder;
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::{buildpack_main, Buildpack};
use libherokubuildpack::log::{log_header, log_info};
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
        log_header(".NET SDK");
        let _sdk_layer = layers::sdk::handle(&context)?;

        BuildResultBuilder::new().build()
    }
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
    #[error("Error reading global.json file")]
    ReadGlobalJsonFile(io::Error),
    #[error("Error parsing global.json file: {0}")]
    ParseGlobalJson(GlobalJsonError),
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
