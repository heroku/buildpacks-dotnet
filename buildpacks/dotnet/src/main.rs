mod detect;
mod dotnet_layer_env;
mod dotnet_project;
mod dotnet_rid;
mod dotnet_solution;
mod global_json;
mod launch_process;
mod layers;
mod tfm;
mod utils;

use crate::dotnet_project::DotnetProject;
use crate::dotnet_rid::RuntimeIdentifier;
use crate::dotnet_solution::DotnetSolution;
use crate::global_json::GlobalJson;
use crate::launch_process::LaunchProcessDetectionError;
use crate::layers::sdk::SdkLayerError;
use crate::tfm::{ParseTargetFrameworkError, TargetFrameworkMoniker};
use crate::utils::StreamedCommandError;
use inventory::artifact::{Arch, Os};
use inventory::inventory::{Inventory, ParseInventoryError};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::LaunchBuilder;
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer::{
    CachedLayerDefinition, InvalidMetadataAction, LayerState, RestoredLayerAction,
};
use libcnb::layer_env::Scope;
use libcnb::{buildpack_main, Buildpack, Env};
use libherokubuildpack::log::{log_header, log_info, log_warning};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sha2::Sha512;
use std::env::consts;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, io};

struct DotnetBuildpack;

impl Buildpack for DotnetBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = DotnetBuildpackError;

    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        if detect::any_dotnet_files(&context.app_dir)? {
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

        // Solution may be an "ephemeral" solution when only a project file is found in the root directory.
        let solution = get_solution_to_publish(&context.app_dir)?;
        log_info(format!(
            "Detected .NET file to publish: {}",
            solution.path.to_string_lossy()
        ));

        let sdk_version_requirement = if let Some(file) = detect::global_json_file(&context.app_dir)
        {
            log_info("Detected global.json file in the root directory");
            VersionReq::try_from(
                fs::read_to_string(file.as_path())
                    .map_err(DotnetBuildpackError::ReadGlobalJsonFile)?
                    .parse::<GlobalJson>()
                    .map_err(DotnetBuildpackError::ParseGlobalJson)?,
            )
            .map_err(DotnetBuildpackError::ParseGlobalJsonVersionRequirement)?
        } else {
            parse_sdk_version_req_for(&solution)?
        };

        log_info(format!(
            "Inferred SDK version requirement: {}",
            &sdk_version_requirement
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
                &sdk_version_requirement
            )
            .ok_or(DotnetBuildpackError::ResolveSdkVersion(sdk_version_requirement))?;

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
                invalid_metadata_action: &|_| InvalidMetadataAction::DeleteLayer,
                restored_layer_action: &|_metadata: &NugetCacheLayerMetadata, _path| {
                    RestoredLayerAction::KeepLayer
                },
            },
        )?;

        match nuget_cache_layer.state {
            LayerState::Restored { .. } => log_info("Reusing NuGet package cache"),
            LayerState::Empty { .. } => {
                log_info("Empty NuGet package cache");
                nuget_cache_layer.write_metadata(NugetCacheLayerMetadata {
                    version: String::from("foo"),
                })?;
            }
        }

        log_header("Publish");

        let command_env = sdk_layer.read_env()?.chainable_insert(
            Scope::Build,
            libcnb::layer_env::ModificationBehavior::Override,
            "NUGET_PACKAGES",
            nuget_cache_layer.path(),
        );

        let configuration = String::from("Release"); // TODO: Make publish configuration configurable;
        let runtime_identifier = dotnet_rid::get_runtime_identifier();
        let launch_processes_result = launch_process::detect_solution_processes(
            &solution,
            &configuration,
            &runtime_identifier,
        );

        utils::run_command_and_stream_output(
            Command::from(PublishCommand {
                path: solution.path,
                configuration,
                runtime_identifier,
                verbosity_level: String::from("normal"),
            })
            .current_dir(&context.app_dir)
            .envs(&command_env.apply(Scope::Build, &Env::from_current())),
        )
        .map_err(DotnetBuildpackError::PublishCommand)?;

        layers::runtime::handle(&context, &sdk_layer.path())?;

        BuildResultBuilder::new()
            .launch(
                LaunchBuilder::new()
                    .processes(
                        launch_processes_result
                            // TODO: Failing to detect launch processes probably shouldn't cause a buildpack error.
                            // Handle errors in a way that provides helpful information to correct the issue, or
                            // instructions for writing a Procfile.
                            .map_err(DotnetBuildpackError::LaunchProcessDetection)?,
                    )
                    .build(),
            )
            .build()
    }
}

struct PublishCommand {
    path: PathBuf,
    configuration: String,
    runtime_identifier: RuntimeIdentifier,
    verbosity_level: String, // TODO: Refactor verbosity level into an enum
}

impl From<PublishCommand> for Command {
    fn from(value: PublishCommand) -> Self {
        let mut command = Command::new("dotnet");
        command.args([
            "publish",
            &value.path.to_string_lossy(),
            "--configuration",
            &value.configuration,
            "--runtime",
            &value.runtime_identifier.to_string(),
            "--verbosity",
            &value.verbosity_level,
        ]);
        command
    }
}

fn get_solution_to_publish(app_dir: &Path) -> Result<DotnetSolution, DotnetBuildpackError> {
    let solution_file_paths =
        detect::solution_file_paths(app_dir).expect("function to pass after detection");
    if let Some(solution_file) = solution_file_paths.first() {
        if solution_file_paths.len() > 1 {
            log_warning(
                "Multiple .NET solution files detected",
                format!(
                    "Found multiple .NET solution files: {}",
                    solution_file_paths
                        .iter()
                        .map(|f| f.to_string_lossy().to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
            );
        }
        return DotnetSolution::load_from_path(solution_file)
            .map_err(DotnetBuildpackError::LoadDotnetSolutionFile);
    }

    let project_file_paths =
        detect::project_file_paths(app_dir).expect("function to pass after detection");
    if let Some(project_file) = detect::project_file_paths(app_dir)
        .expect("function to pass after detection")
        .first()
    {
        if project_file_paths.len() > 1 {
            return Err(DotnetBuildpackError::MultipleProjectFiles(
                project_file_paths
                    .iter()
                    .map(|f| f.to_string_lossy().to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            ));
        }
        return Ok(DotnetSolution::ephemeral(
            DotnetProject::load_from_path(project_file)
                .map_err(DotnetBuildpackError::LoadDotnetProjectFile)?,
        ));
    }

    Err(DotnetBuildpackError::NoDotnetFiles)
}

fn parse_sdk_version_req_for(
    solution: &DotnetSolution,
) -> Result<VersionReq, DotnetBuildpackError> {
    let requirements = solution
        .projects
        .iter()
        .map(|project| {
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
        })
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
    #[error("Error loading .NET solution file")]
    LoadDotnetSolutionFile(dotnet_solution::LoadSolutionError),
    #[error("Error loading .NET project file")]
    LoadDotnetProjectFile(dotnet_project::LoadProjectError),
    #[error("Error parsing target framework moniker: {0}")]
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
    #[error("Error executing publish task")]
    PublishCommand(#[from] StreamedCommandError),
    #[error("Error copying runtime files {0}")]
    CopyRuntimeFilesToRuntimeLayer(io::Error),
    #[error("Launch process detection error: {0}")]
    LaunchProcessDetection(LaunchProcessDetectionError),
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
