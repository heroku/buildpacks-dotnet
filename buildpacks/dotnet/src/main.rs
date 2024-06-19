mod detect;
mod dotnet_layer_env;
mod dotnet_project;
mod dotnet_publish_command;
mod dotnet_rid;
mod dotnet_solution;
mod global_json;
mod launch_process;
mod layers;
mod tfm;
mod utils;

use crate::dotnet_project::DotnetProject;
use crate::dotnet_publish_command::{DotnetPublishCommand, VerbosityLevel};
use crate::dotnet_solution::DotnetSolution;
use crate::global_json::GlobalJson;
use crate::launch_process::LaunchProcessDetectionError;
use crate::layers::sdk::SdkLayerError;
use crate::tfm::{ParseTargetFrameworkError, TargetFrameworkMoniker};
use crate::utils::StreamedCommandError;
use inventory::artifact::{Arch, Artifact, Os};
use inventory::inventory::{Inventory, ParseInventoryError};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::LaunchBuilder;
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer::{
    CachedLayerDefinition, InvalidMetadataAction, LayerRef, LayerState, RestoredLayerAction,
};
use libcnb::layer_env::Scope;
use libcnb::{buildpack_main, Buildpack, Env, Target};
use libherokubuildpack::log::{log_header, log_info, log_warning};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sha2::Sha512;
use std::path::Path;
use std::process::Command;
use std::{fs, io};

struct DotnetBuildpack;

impl Buildpack for DotnetBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = DotnetBuildpackError;

    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        if detect::any_dotnet_files(&context.app_dir)? {
            DetectResultBuilder::pass().build()
        } else {
            log_info(
                "No .NET solution or project files (such as `foo.sln` or `foo.csproj`) found.",
            );
            DetectResultBuilder::fail().build()
        }
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        log_header("Determining .NET version");
        let solution = get_solution_to_publish(&context.app_dir)?; // Solution may be an "ephemeral" solution when only a project file is found in the root directory.
        log_info(format!(
            "Detected .NET file to publish: {}",
            solution.path.to_string_lossy()
        ));

        let sdk_version_requirement = detect_global_json_sdk_version_requirement(&context.app_dir)
            .unwrap_or_else(|| get_solution_sdk_version_requirement(&solution))?;
        log_info(format!(
            "Inferred SDK version requirement: {sdk_version_requirement}",
        ));
        let sdk_artifact = resolve_sdk_artifact(&context.target, sdk_version_requirement)?;
        log_info(format!(
            "Resolved .NET SDK version {} ({}-{})",
            sdk_artifact.version, sdk_artifact.os, sdk_artifact.arch
        ));
        let sdk_layer = layers::sdk::handle(&context, &sdk_artifact)?;

        let nuget_cache_layer = handle_nuget_cache_layer(&context)?;

        log_header("Publish");
        let build_configuration = String::from("Release");
        let runtime_identifier = dotnet_rid::get_runtime_identifier();
        let command_env = sdk_layer.read_env()?.chainable_insert(
            Scope::Build,
            libcnb::layer_env::ModificationBehavior::Override,
            "NUGET_PACKAGES",
            nuget_cache_layer.path(),
        );

        let launch_processes_result = launch_process::detect_solution_processes(
            &solution,
            &build_configuration,
            &runtime_identifier,
        )
        .map_err(DotnetBuildpackError::LaunchProcessDetection);

        utils::run_command_and_stream_output(
            Command::from(DotnetPublishCommand {
                path: solution.path,
                configuration: build_configuration,
                runtime_identifier,
                verbosity_level: VerbosityLevel::Normal,
            })
            .current_dir(&context.app_dir)
            .envs(&command_env.apply(Scope::Build, &Env::from_current())),
        )
        .map_err(DotnetBuildpackError::PublishCommand)?;

        layers::runtime::handle(&context, &sdk_layer.path())?;

        BuildResultBuilder::new()
            .launch(
                LaunchBuilder::new()
                    .processes(launch_processes_result?)
                    .build(),
            )
            .build()
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

fn get_solution_sdk_version_requirement(
    solution: &DotnetSolution,
) -> Result<VersionReq, DotnetBuildpackError> {
    let mut target_framework_monikers = solution
        .projects
        .iter()
        .map(|project| {
            log_info(format!(
                "Detecting target framework for project {0}",
                project.path.to_string_lossy()
            ));
            project
                .target_framework
                .parse::<TargetFrameworkMoniker>()
                .map_err(DotnetBuildpackError::ParseTargetFrameworkMoniker)
        })
        .collect::<Result<Vec<_>, _>>()?;
    target_framework_monikers.sort_by_key(|tfm| tfm.version_part.clone());

    VersionReq::try_from(
        target_framework_monikers
            .first()
            .ok_or(DotnetBuildpackError::NoDotnetFiles)?,
    )
    .map_err(DotnetBuildpackError::ParseVersionRequirement)
}

fn detect_global_json_sdk_version_requirement(
    app_dir: &Path,
) -> Option<Result<VersionReq, DotnetBuildpackError>> {
    detect::global_json_file(app_dir).map(|file| {
        log_info("Detected global.json file in the root directory");
        VersionReq::try_from(
            fs::read_to_string(file.as_path())
                .map_err(DotnetBuildpackError::ReadGlobalJsonFile)?
                .parse::<GlobalJson>()
                .map_err(DotnetBuildpackError::ParseGlobalJson)?,
        )
        .map_err(DotnetBuildpackError::ParseGlobalJsonVersionRequirement)
    })
}

fn resolve_sdk_artifact(
    target: &Target,
    sdk_version_requirement: VersionReq,
) -> Result<Artifact<Version, Sha512, Option<()>>, DotnetBuildpackError> {
    let inventory = include_str!("../inventory.toml")
        .parse::<Inventory<Version, Sha512, Option<()>>>()
        .map_err(DotnetBuildpackError::ParseInventory)?;

    inventory
        .resolve(
            target.os
                .parse::<Os>()
                .expect("OS should be always parseable, buildpack will not run on unsupported operating systems."),
            target.arch
                .parse::<Arch>()
                .expect("Arch should be always parseable, buildpack will not run on unsupported architectures."),
            &sdk_version_requirement
        )
        .ok_or(DotnetBuildpackError::ResolveSdkVersion(sdk_version_requirement)).cloned()
}

#[derive(Serialize, Deserialize)]
struct NugetCacheLayerMetadata {
    version: String,
}

fn handle_nuget_cache_layer(
    context: &BuildContext<DotnetBuildpack>,
) -> Result<LayerRef<DotnetBuildpack, (), ()>, libcnb::Error<<DotnetBuildpack as Buildpack>::Error>>
{
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
    Ok(nuget_cache_layer)
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
