mod detect;
mod dotnet_layer_env;
mod dotnet_project;
mod dotnet_rid;
mod dotnet_solution;
mod global_json;
mod layers;
mod tfm;
mod utils;

use crate::dotnet_project::DotnetProject;
use crate::dotnet_solution::DotnetSolution;
use crate::global_json::GlobalJson;
use crate::layers::sdk::SdkLayerError;
use crate::tfm::{ParseTargetFrameworkError, TargetFrameworkMoniker};
use crate::utils::StreamedCommandError;
use inventory::artifact::{Arch, Os};
use inventory::inventory::{Inventory, ParseInventoryError};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer::{
    CachedLayerDefinition, InspectRestoredAction, InvalidMetadataAction, LayerState,
};
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::{buildpack_main, Buildpack, Env};
use libherokubuildpack::log::{log_header, log_info, log_warning};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sha2::Sha512;
use std::env::consts;
use std::path::Path;
use std::process::Command;
use std::{fs, io};

struct DotnetBuildpack;

impl Buildpack for DotnetBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = DotnetBuildpackError;

    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        let solution_files = detect::dotnet_solution_files(&context.app_dir)
            .map_err(DotnetBuildpackError::BuildpackDetection)?;
        let project_files = detect::dotnet_project_files(&context.app_dir)
            .map_err(DotnetBuildpackError::BuildpackDetection)?;

        if solution_files.is_empty() && project_files.is_empty() {
            log_info(
                "No .NET solution or project files (such as `foo.sln` or `foo.csproj`) found.",
            );
            DetectResultBuilder::fail().build()
        } else {
            DetectResultBuilder::pass().build()
        }
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        log_header("Determining .NET version");

        let dotnet_file = determine_file_to_publish(&context.app_dir)?;
        log_info(format!(
            "Detected .NET file to publish: {}",
            dotnet_file.path.to_string_lossy()
        ));

        let sdk_version_req = if let Some(file) = detect::find_global_json(&context.app_dir) {
            log_info("Detected global.json file in the root directory");
            VersionReq::try_from(
                fs::read_to_string(file.as_path())
                    .map_err(DotnetBuildpackError::ReadGlobalJsonFile)?
                    .parse::<GlobalJson>()
                    .map_err(DotnetBuildpackError::ParseGlobalJson)?,
            )
            .map_err(DotnetBuildpackError::ParseGlobalJsonVersionRequirement)?
        } else {
            get_version_requirement_from_dotnet_file(&dotnet_file)?
        };

        log_info(format!(
            "Inferred SDK version requirement: {}",
            &sdk_version_req
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
                &sdk_version_req
            )
            .ok_or(DotnetBuildpackError::ResolveSdkVersion(sdk_version_req))?;

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
                inspect_restored: &|_metadata: &NugetCacheLayerMetadata, _path| {
                    InspectRestoredAction::KeepLayer
                },
            },
        )?;

        match nuget_cache_layer.state {
            LayerState::Restored { .. } => log_info("Reusing NuGet package cache"),
            LayerState::Empty { .. } => {
                log_info("Empty NuGet package cache");
                nuget_cache_layer.replace_metadata(NugetCacheLayerMetadata {
                    version: String::from("foo"),
                })?;
            }
        }

        log_header("Publish");

        let command_env = LayerEnv::read_from_layer_dir(sdk_layer.path())
            .map_err(DotnetBuildpackError::ReadSdkLayerEnvironment)?
            .chainable_insert(
                Scope::Build,
                libcnb::layer_env::ModificationBehavior::Override,
                "NUGET_PACKAGES",
                nuget_cache_layer.path(),
            );
        utils::run_command_and_stream_output(
            Command::new("dotnet")
                .args([
                    "publish",
                    &dotnet_file.path.to_string_lossy(),
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

        layers::runtime::handle(&context, &sdk_layer.path())?;

        BuildResultBuilder::new().build()
    }
}

fn determine_file_to_publish(app_dir: &Path) -> Result<DotnetSolution, DotnetBuildpackError> {
    let solution_files =
        detect::dotnet_solution_files(app_dir).expect("function to pass after detection");
    let project_files =
        detect::dotnet_project_files(app_dir).expect("function to pass after detection");

    match (solution_files.first(), project_files.first()) {
        (Some(solution_file), _) => {
            if solution_files.len() > 1 {
                log_warning(
                    "Multiple .NET solution files detected",
                    format!(
                        "Found multiple .NET solution files: {}",
                        solution_files
                            .iter()
                            .map(|f| f.to_string_lossy().to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    ),
                );
            }
            DotnetSolution::load_from_path(solution_file)
        }
        (None, Some(project_file)) => {
            if project_files.len() > 1 {
                return Err(DotnetBuildpackError::MultipleProjectFiles(
                    project_files
                        .iter()
                        .map(|f| f.to_string_lossy().to_string())
                        .collect::<Vec<String>>()
                        .join(", "),
                ));
            }
            Ok(DotnetSolution::ephemeral(DotnetProject::load_from_path(
                project_file,
            )?))
        }
        (None, None) => Err(DotnetBuildpackError::NoDotnetFiles),
    }
}

fn parse_project_sdk_version_requirement(
    project: &DotnetProject,
) -> Result<VersionReq, DotnetBuildpackError> {
    log_info(format!(
        "Detecting .NET version requirement for project {0}",
        project.path.to_string_lossy()
    ));

    VersionReq::try_from(
        project
            .target_framework
            .parse::<TargetFrameworkMoniker>()
            .map_err(DotnetBuildpackError::ParseTargetFrameworkMoniker)?,
    )
    .map_err(DotnetBuildpackError::ParseVersionRequirement)
}

fn get_version_requirement_from_dotnet_file(
    dotnet_file: &DotnetSolution,
) -> Result<VersionReq, DotnetBuildpackError> {
    let requirements = dotnet_file
        .projects
        .iter()
        .map(parse_project_sdk_version_requirement)
        .collect::<Result<Vec<_>, _>>()?;

    requirements
        // TODO: Add logic to prefer the most recent version requirement, and log if projects target different versions
        .first()
        .cloned()
        .ok_or(DotnetBuildpackError::NoDotnetFiles)
}

#[derive(Serialize, Deserialize)]
struct NugetCacheLayerMetadata {
    version: String,
}

#[derive(thiserror::Error, Debug)]
enum DotnetBuildpackError {
    #[error("Error when performing buildpack detection")]
    BuildpackDetection(io::Error),
    #[error("No .NET solution or project files found")]
    NoDotnetFiles,
    #[error("Multiple .NET project files found in root directory: {0}")]
    MultipleProjectFiles(String),
    #[error("Error reading .NET file")]
    ReadDotnetFile(io::Error),
    #[error("Error parsing .NET project file")]
    ParseDotnetProjectFile(dotnet_project::ParseError),
    #[error("Error parsing solution file: {0}")]
    ParseTargetFrameworkMoniker(ParseTargetFrameworkError),
    #[error("Error reading global.json file")]
    ReadGlobalJsonFile(io::Error),
    #[error("Error parsing global.json: {0}")]
    ParseGlobalJson(serde_json::Error),
    #[error("Error parsing global.json version requirement: {0}")]
    ParseGlobalJsonVersionRequirement(semver::Error),
    #[error("Couldn't parse .NET SDK inventory: {0}")]
    ParseInventory(ParseInventoryError),
    #[error("Invalid target framework version: {0}")]
    ParseVersionRequirement(semver::Error),
    #[error("Couldn't resolve .NET SDK version: {0}")]
    ResolveSdkVersion(VersionReq),
    #[error(transparent)]
    SdkLayer(#[from] SdkLayerError),
    #[error("Error reading SDK layer environment")]
    ReadSdkLayerEnvironment(io::Error),
    #[error("Error executing publish task")]
    PublishCommand(#[from] StreamedCommandError),
    #[error("Error copying runtime files {0}")]
    CopyRuntimeFilesToRuntimeLayer(io::Error),
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
