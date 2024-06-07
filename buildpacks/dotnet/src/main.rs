mod detect;
mod dotnet_project;
mod dotnet_rid;
mod global_json;
mod layers;
mod tfm;
mod utils;

use crate::dotnet_project::DotnetProject;
use crate::global_json::GlobalJsonError;
use crate::layers::sdk::SdkLayerError;
use crate::tfm::ParseTargetFrameworkError;
use crate::utils::StreamedCommandError;
use fs_extra::{copy_items, dir};
use inventory::artifact::{Arch, Os};
use inventory::inventory::{Inventory, ParseInventoryError};
use libcnb::build::BuildResultBuilder;
use libcnb::data::layer_name;
use libcnb::detect::DetectResultBuilder;
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer::{
    CachedLayerDefinition, InspectExistingAction, InvalidMetadataAction, UncachedLayerDefinition,
};
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::{buildpack_main, Buildpack, Env};
use libherokubuildpack::log::{log_header, log_info};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::Sha512;
use std::env::consts;
use std::path::PathBuf;
use std::process::Command;
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

    #[allow(clippy::too_many_lines)]
    fn build(
        &self,
        context: libcnb::build::BuildContext<Self>,
    ) -> libcnb::Result<libcnb::build::BuildResult, Self::Error> {
        log_header("Determining .NET version");

        // TODO: Implement and document the project/solution file selection logic
        let project_files = detect::dotnet_project_files(context.app_dir.clone())
            .expect("function to pass after detection");

        let dotnet_project_file = project_files.first().expect("a project file to be present");

        log_info(format!(
            "Detected .NET project file: {}",
            dotnet_project_file.to_string_lossy()
        ));

        let requirement = if let Some(file) = detect::find_global_json(context.app_dir.clone()) {
            log_info("Detected global.json file in the root directory");

            fs::read_to_string(file.as_path())
                .map_err(DotnetBuildpackError::ReadGlobalJsonFile)
                .map(|content| global_json::parse_global_json(&content))?
                .map_err(DotnetBuildpackError::ParseGlobalJson)?
        } else {
            let dotnet_project = fs::read_to_string(dotnet_project_file)
                .map_err(DotnetBuildpackError::ReadProjectFile)?
                .parse::<DotnetProject>()
                .map_err(DotnetBuildpackError::ParseDotnetProjectFile)?;

            // TODO: Remove this (currently here for debugging, and making the linter happy)
            log_info(format!("Project type is {:?} using SDK \"{}\" specifies TFM \"{}\" and assembly name \"{}\"",
                dotnet_project.project_type,
                dotnet_project.sdk_id,
                dotnet_project.target_framework,
                dotnet_project.assembly_name.unwrap_or(String::new())
            ));
            tfm::parse_target_framework(&dotnet_project.target_framework)
                .map_err(DotnetBuildpackError::ParseTargetFramework)?
        };

        log_info(format!(
            "Inferred SDK version requirement: {}",
            &requirement.to_string()
        ));

        let inventory = include_str!("../inventory.toml")
            .parse::<Inventory<Version, Sha512, Option<()>>>()
            .map_err(DotnetBuildpackError::ParseInventory)?;

        let artifact = inventory
            .resolve(
                consts::OS
                    .parse::<Os>()
                    .expect("OS should be always parseable, buildpack will not run on unsupported operating systems."),
                consts::ARCH
                    .parse::<Arch>()
                    .expect("Arch should be always parseable, buildpack will not run on unsupported architectures."),
                &requirement
            )
            .ok_or(DotnetBuildpackError::ResolveSdkVersion(requirement))?;

        log_info(format!(
            "Resolved .NET SDK version {} ({}-{})",
            artifact.version, artifact.os, artifact.arch
        ));

        let sdk_layer = layers::sdk::handle(&context, artifact)?;

        let nuget_cache_layer = context.cached_layer(
            layer_name!("nuget-cache"),
            CachedLayerDefinition {
                build: false,
                launch: false,
                invalid_metadata: &|_| {
                    log_info("Invalid NuGet package cache");
                    InvalidMetadataAction::DeleteLayer
                },
                inspect_existing: &|_metadata: &NugetCacheLayerMetadata, _path| {
                    InspectExistingAction::Keep
                },
            },
        )?;

        match nuget_cache_layer.contents {
            libcnb::layer::LayerContents::Cached(()) => log_info("Reusing NuGet package cache"),
            libcnb::layer::LayerContents::Empty(_) => {
                log_info("Empty NuGet package cache");
                nuget_cache_layer.replace_metadata(NugetCacheLayerMetadata {
                    // TODO: Implement cache expiration/purging logic
                    version: String::from("foo"),
                })?;
            }
        }

        let command_env = LayerEnv::read_from_layer_dir(sdk_layer.path())
            .map_err(DotnetBuildpackError::ReadSdkLayerEnvironment)?
            .chainable_insert(
                Scope::Build,
                libcnb::layer_env::ModificationBehavior::Override,
                "NUGET_PACKAGES",
                nuget_cache_layer.path(),
            );

        log_header("Publish");
        utils::run_command_and_stream_output(
            Command::new("dotnet")
                .args([
                    "publish",
                    &dotnet_project_file.to_string_lossy(),
                    "--verbosity",
                    "normal",
                    "--configuration",
                    "Release",
                    "--runtime",
                    &dotnet_rid::get_runtime_identifier().to_string(),
                ])
                .current_dir(&context.app_dir)
                .envs(&command_env.apply(Scope::Build, &Env::from_current())),
        )
        .map_err(DotnetBuildpackError::PublishCommand)?;

        // Copy the runtime files to it's own layer to reduce final image size.
        // TODO: Move this logic to it's own module or function, or combine with
        // sdk layer module.
        let runtime_layer = context.uncached_layer(
            layer_name!("runtime"),
            UncachedLayerDefinition {
                build: false,
                launch: true,
            },
        )?;
        runtime_layer.replace_env(&layers::sdk::generate_layer_env(&runtime_layer.path()))?;

        let runtime_paths: Vec<PathBuf> = [
            "dotnet",
            "host",
            "shared",
            "ThirdPartyNotices.txt",
            "LICENSE.txt",
        ]
        .iter()
        .map(|path| sdk_layer.path().join(path))
        .collect();

        copy_items(
            &runtime_paths,
            runtime_layer.path(),
            &dir::CopyOptions {
                copy_inside: true,
                ..Default::default()
            },
        )
        .map_err(DotnetBuildpackError::CopyRuntimeFilesToRuntimeLayer)?;

        BuildResultBuilder::new().build()
    }
}

#[derive(Serialize, Deserialize)]
struct NugetCacheLayerMetadata {
    version: String,
}

#[derive(thiserror::Error, Debug)]
enum DotnetBuildpackError {
    #[error("Error when performing buildpack detection")]
    BuildpackDetection(io::Error),
    #[error("Couldn't parse .NET SDK inventory: {0}")]
    ParseInventory(ParseInventoryError),
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
    #[error(transparent)]
    SdkLayer(#[from] SdkLayerError),
    #[error("Error reading SDK layer environment")]
    ReadSdkLayerEnvironment(io::Error),
    #[error("Error executing publish task")]
    PublishCommand(#[from] StreamedCommandError),
    #[error("Error copying runtime files {0}")]
    CopyRuntimeFilesToRuntimeLayer(fs_extra::error::Error),
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
