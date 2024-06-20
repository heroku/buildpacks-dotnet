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
use inventory::artifact::{Arch, Os};
use inventory::inventory::{Inventory, ParseInventoryError};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::LaunchBuilder;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer_env::Scope;
use libcnb::{buildpack_main, Buildpack, Env};
use libherokubuildpack::log::{log_header, log_info, log_warning};
use semver::{Version, VersionReq};
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

        let target_os = context.target.os.parse::<Os>()
            .expect("OS should always be parseable, buildpack will not run on unsupported operating systems.");
        let target_arch = context.target.arch.parse::<Arch>().expect(
            "Arch should always be parseable, buildpack will not run on unsupported architectures.",
        );

        let sdk_artifact = {
            let inventory = include_str!("../inventory.toml")
                .parse::<Inventory<Version, Sha512, Option<()>>>()
                .map_err(DotnetBuildpackError::ParseInventory)?;

            inventory
                .resolve(target_os, target_arch, &sdk_version_requirement)
                .ok_or(DotnetBuildpackError::ResolveSdkVersion(
                    sdk_version_requirement,
                ))
                .cloned()
        }?;
        log_info(format!(
            "Resolved .NET SDK version {} ({}-{})",
            sdk_artifact.version, sdk_artifact.os, sdk_artifact.arch
        ));
        let sdk_layer = layers::sdk::handle(&context, &sdk_artifact)?;

        let nuget_cache_layer = layers::nuget_cache::handle(&context)?;

        log_header("Publish");
        let build_configuration = String::from("Release");
        let runtime_identifier = dotnet_rid::get_runtime_identifier(target_os, target_arch);
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
                "Parsing target framework moniker for project: {0}",
                project.path.to_string_lossy()
            ));
            project
                .target_framework
                .parse::<TargetFrameworkMoniker>()
                .map_err(DotnetBuildpackError::ParseTargetFrameworkMoniker)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // The target framework monikers are sorted lexicographically, which is sufficient for now
    // (as the only expected TFMs are currently "net5.0", "net6.0", "net7.0", "net8.0", "net9.0").
    target_framework_monikers.sort_by_key(|tfm| tfm.version_part.clone());

    VersionReq::try_from(
        target_framework_monikers
            // The last (i.e. most recent, based on the sorting logic above) target framework moniker is preferred
            .last()
            .ok_or(DotnetBuildpackError::NoSolutionProjects)?,
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

#[derive(thiserror::Error, Debug)]
enum DotnetBuildpackError {
    #[error("Error when performing buildpack detection")]
    BuildpackDetection(io::Error),
    #[error("No .NET solution or project files found")]
    NoDotnetFiles,
    #[error("No project references found in solution")]
    NoSolutionProjects,
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
