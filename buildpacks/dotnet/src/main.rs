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
use crate::global_json::GlobalJsonError;
use crate::layers::sdk::SdkLayerError;
use crate::tfm::ParseTargetFrameworkError;
use crate::utils::StreamedCommandError;
use inventory::artifact::{Arch, Os};
use inventory::inventory::{Inventory, ParseInventoryError};
use libcnb::build::BuildResultBuilder;
use libcnb::data::layer_name;
use libcnb::detect::DetectResultBuilder;
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer::{CachedLayerDefinition, InspectExistingAction, InvalidMetadataAction};
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

    fn detect(
        &self,
        context: libcnb::detect::DetectContext<Self>,
    ) -> libcnb::Result<libcnb::detect::DetectResult, Self::Error> {
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

    #[allow(clippy::too_many_lines)]
    fn build(
        &self,
        context: libcnb::build::BuildContext<Self>,
    ) -> libcnb::Result<libcnb::build::BuildResult, Self::Error> {
        log_header("Determining .NET version");

        let solution_files = detect::dotnet_solution_files(&context.app_dir)
            .expect("function to pass after detection");
        let project_files = detect::dotnet_project_files(&context.app_dir)
            .expect("function to pass after detection");

        let (file_to_publish, requirement) = if !solution_files.is_empty() {
            let dotnet_solution_file = solution_files
                .first()
                .expect("a solution file to be present");

            // TODO: Publish all solutions instead of just the first
            if solution_files.len() > 1 {
                log_warning(
                    "Multiple .NET solution files detected",
                    format!(
                        "Found multiple .NET solution files: {}. Publishing the first one.",
                        solution_files
                            .iter()
                            .map(|f| f.to_string_lossy().to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    ),
                );
            }

            log_info(format!(
                "Detected .NET solution file: {}",
                dotnet_solution_file.to_string_lossy()
            ));

            let mut version_requirements = vec![];
            for project_reference in dotnet_solution::project_file_paths(dotnet_solution_file)
                .map_err(DotnetBuildpackError::ParseDotnetSolutionFile)?
            {
                log_info(format!(
                    "Detecting .NET version requirement for project {project_reference}"
                ));

                version_requirements.push(get_requirement_from_project_file(
                    &dotnet_solution_file
                        .parent()
                        .expect("solution file to have a parent directory")
                        .join(project_reference),
                )?);
            }

            (
                dotnet_solution_file,
                version_requirements
                    // TODO: Add logic to prefer the most recent version requirement, and log if projects target different versions
                    .first()
                    // TODO: Handle solution files with no project references
                    .expect("at least one project in solution")
                    .clone(),
            )
        } else if !project_files.is_empty() {
            if project_files.len() > 1 {
                return Err(DotnetBuildpackError::MultipleProjectFiles(
                    project_files
                        .iter()
                        .map(|f| f.to_string_lossy().to_string())
                        .collect::<Vec<String>>()
                        .join(", "),
                )
                .into());
            }
            let dotnet_project_file = project_files.first().expect("a project file to be present");
            log_info(format!(
                "Detected .NET project file: {}",
                dotnet_project_file.to_string_lossy()
            ));
            (
                dotnet_project_file,
                get_requirement_from_project_file(dotnet_project_file)?,
            )
        } else {
            // This error is not expected to occur (as one or more solution/project files should be present after detect())
            return Err(DotnetBuildpackError::NoDotnetFiles)?;
        };

        let requirement = if let Some(file) = detect::find_global_json(&context.app_dir) {
            log_info("Detected global.json file in the root directory");

            fs::read_to_string(file.as_path())
                .map_err(DotnetBuildpackError::ReadGlobalJsonFile)
                .map(|content| global_json::parse_global_json(&content))?
                .map_err(DotnetBuildpackError::ParseGlobalJson)?
        } else {
            requirement
        };

        log_info(format!(
            "Inferred SDK version requirement: {}",
            &requirement
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
                    &file_to_publish.to_string_lossy(),
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

fn get_requirement_from_project_file(
    dotnet_project_file: &Path,
) -> Result<VersionReq, DotnetBuildpackError> {
    let dotnet_project = fs::read_to_string(dotnet_project_file)
        .map_err(DotnetBuildpackError::ReadProjectFile)?
        .parse::<DotnetProject>()
        .map_err(DotnetBuildpackError::ParseDotnetProjectFile)?;

    log_info(format!(
        "Project type is {:?} using SDK \"{}\" specifies TFM \"{}\" and assembly name \"{}\"",
        dotnet_project.project_type,
        dotnet_project.sdk_id,
        dotnet_project.target_framework,
        dotnet_project.assembly_name.unwrap_or_default()
    ));
    tfm::parse_target_framework(&dotnet_project.target_framework)
        .map_err(DotnetBuildpackError::ParseTargetFramework)
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
    ResolveSdkVersion(VersionReq),
    #[error("Error reading project file")]
    ReadProjectFile(io::Error),
    #[error("Error parsing .NET project file")]
    ParseDotnetProjectFile(dotnet_project::ParseError),
    #[error("Error parsing target framework: {0}")]
    ParseDotnetSolutionFile(io::Error),
    #[error("Error parsing solution file: {0}")]
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
    #[error("No .NET solution or project files found")]
    NoDotnetFiles,
    #[error("Multiple .NET project files found in root directory: {0}")]
    MultipleProjectFiles(String),
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
