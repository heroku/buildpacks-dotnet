mod detect;
mod dotnet;
mod dotnet_buildpack_configuration;
mod dotnet_layer_env;
mod dotnet_publish_command;
mod errors;
mod launch_process;
mod layers;
mod utils;

use crate::dotnet::global_json::GlobalJson;
use crate::dotnet::project::Project;
use crate::dotnet::runtime_identifier;
use crate::dotnet::solution::Solution;
use crate::dotnet::target_framework_moniker::{ParseTargetFrameworkError, TargetFrameworkMoniker};
use crate::dotnet_buildpack_configuration::{
    DotnetBuildpackConfiguration, DotnetBuildpackConfigurationError,
};
use crate::dotnet_publish_command::DotnetPublishCommand;
use crate::launch_process::LaunchProcessDetectionError;
use crate::layers::sdk::SdkLayerError;
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
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, io};

struct DotnetBuildpack;

impl Buildpack for DotnetBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = DotnetBuildpackError;

    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        let contains_dotnet_files = detect::get_files_with_extensions(
            &context.app_dir,
            &["sln", "csproj", "vbproj", "fsproj"],
        )
        .map(|paths| !paths.is_empty())
        .map_err(DotnetBuildpackError::BuildpackDetection)?;

        if contains_dotnet_files {
            DetectResultBuilder::pass().build()
        } else {
            log_info(
                "No .NET solution or project files (such as `foo.sln` or `foo.csproj`) found.",
            );
            DetectResultBuilder::fail().build()
        }
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        let buildpack_configuration = DotnetBuildpackConfiguration::try_from(&Env::from_current())
            .map_err(DotnetBuildpackError::ParseBuildpackConfiguration)?;

        log_header("Determining .NET version");
        let solution = get_solution_to_publish(&context.app_dir)?;
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

        let sdk_inventory = include_str!("../inventory.toml")
            .parse::<Inventory<Version, Sha512, Option<()>>>()
            .map_err(DotnetBuildpackError::ParseInventory)?;
        let sdk_artifact = sdk_inventory
            .resolve(target_os, target_arch, &sdk_version_requirement)
            .ok_or(DotnetBuildpackError::ResolveSdkVersion(
                sdk_version_requirement,
            ))?;
        log_info(format!(
            "Resolved .NET SDK version {} ({}-{})",
            sdk_artifact.version, sdk_artifact.os, sdk_artifact.arch
        ));
        let sdk_layer = layers::sdk::handle(&context, sdk_artifact)?;

        let nuget_cache_layer = layers::nuget_cache::handle(&context)?;

        log_header("Publish");
        let runtime_identifier = runtime_identifier::get_runtime_identifier(target_os, target_arch);
        let command_env = sdk_layer.read_env()?.chainable_insert(
            Scope::Build,
            libcnb::layer_env::ModificationBehavior::Override,
            "NUGET_PACKAGES",
            nuget_cache_layer.path(),
        );

        let build_configuration = buildpack_configuration
            .build_configuration
            .clone()
            .unwrap_or_else(|| String::from("Release"));

        let launch_processes_result = launch_process::detect_solution_processes(
            &solution,
            &build_configuration,
            &runtime_identifier,
        )
        .map_err(DotnetBuildpackError::LaunchProcessDetection);

        utils::run_command_and_stream_output(
            Command::from(DotnetPublishCommand {
                path: solution.path,
                configuration: buildpack_configuration.build_configuration,
                runtime_identifier,
                verbosity_level: buildpack_configuration.msbuild_verbosity_level,
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

    fn on_error(&self, error: libcnb::Error<Self::Error>) {
        errors::on_error(error);
    }
}

fn get_solution_to_publish(app_dir: &Path) -> Result<Solution, DotnetBuildpackError> {
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
        return Solution::load_from_path(solution_file)
            .map_err(DotnetBuildpackError::LoadSolutionFile);
    }

    let project_file_paths =
        detect::project_file_paths(app_dir).expect("function to pass after detection");
    if project_file_paths.len() > 1 {
        return Err(DotnetBuildpackError::MultipleRootDirectoryProjectFiles(
            project_file_paths,
        ));
    }
    Ok(Solution::ephemeral(
        Project::load_from_path(
            project_file_paths
                .first()
                .expect("A project file to be present"),
        )
        .map_err(DotnetBuildpackError::LoadProjectFile)?,
    ))
}

fn get_solution_sdk_version_requirement(
    solution: &Solution,
) -> Result<VersionReq, DotnetBuildpackError> {
    let mut target_framework_monikers = solution
        .projects
        .iter()
        .map(|project| {
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
    .map_err(DotnetBuildpackError::ParseSolutionVersionRequirement)
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

#[derive(Debug)]
enum DotnetBuildpackError {
    BuildpackDetection(io::Error),
    NoSolutionProjects,
    MultipleRootDirectoryProjectFiles(Vec<PathBuf>),
    LoadSolutionFile(dotnet::solution::LoadError),
    LoadProjectFile(dotnet::project::LoadError),
    ParseTargetFrameworkMoniker(ParseTargetFrameworkError),
    ReadGlobalJsonFile(io::Error),
    ParseGlobalJson(serde_json::Error),
    ParseGlobalJsonVersionRequirement(semver::Error),
    ParseInventory(ParseInventoryError),
    ParseSolutionVersionRequirement(semver::Error),
    ResolveSdkVersion(VersionReq),
    SdkLayer(SdkLayerError),
    ParseBuildpackConfiguration(DotnetBuildpackConfigurationError),
    PublishCommand(StreamedCommandError),
    CopyRuntimeFiles(io::Error),
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
