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
use buildpacks_jvm_shared::output::{
    print_buildpack_name, print_section, print_subsection, print_warning, run_command,
    track_timing, BuildpackOutputTextSection,
};
use indoc::formatdoc;
use inventory::artifact::{Arch, Os};
use inventory::{Inventory, ParseInventoryError};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::LaunchBuilder;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer_env::Scope;
use libcnb::{buildpack_main, Buildpack, Env};
use libherokubuildpack::inventory;
use semver::{Version, VersionReq};
use sha2::Sha512;
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{fs, io};

struct DotnetBuildpack;

impl Buildpack for DotnetBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = DotnetBuildpackError;

    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        detect::get_files_with_extensions(&context.app_dir, &["sln", "csproj", "vbproj", "fsproj"])
            .map(|paths| {
                if paths.is_empty() {
                    println!("No .NET solution or project files (such as `foo.sln` or `foo.csproj`) found.");
                    std::io::stdout().flush().expect("Couldn't flush output stream");
                    DetectResultBuilder::fail().build()
                } else {
                    DetectResultBuilder::pass().build()
                }
            })
            .map_err(DotnetBuildpackError::BuildpackDetection)?
    }

    #[allow(clippy::too_many_lines)]
    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        let buildpack_configuration = DotnetBuildpackConfiguration::try_from(&Env::from_current())
            .map_err(DotnetBuildpackError::ParseBuildpackConfiguration)?;

        print_buildpack_name("Heroku .NET Buildpack");
        print_section("SDK version detection");

        let solution = get_solution_to_publish(&context.app_dir)?;

        print_subsection(vec![
            BuildpackOutputTextSection::regular("Detected .NET file to publish: "),
            BuildpackOutputTextSection::value(solution.path.to_string_lossy()),
        ]);

        let sdk_version_requirement = if let Some(version_req) =
            detect_global_json_sdk_version_requirement(&context.app_dir)
        {
            print_subsection("Detecting version requirement from root global.json file");
            version_req?
        } else {
            print_subsection(vec![
                BuildpackOutputTextSection::regular("Inferring version requirement from "),
                BuildpackOutputTextSection::value(solution.path.to_string_lossy()),
            ]);
            get_solution_sdk_version_requirement(&solution)?
        };

        print_subsection(vec![
            BuildpackOutputTextSection::regular("Detected version requirement: "),
            BuildpackOutputTextSection::value(sdk_version_requirement.to_string()),
        ]);

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

        print_subsection(vec![
            BuildpackOutputTextSection::regular("Resolved .NET SDK version "),
            BuildpackOutputTextSection::value(sdk_artifact.version.to_string()),
            BuildpackOutputTextSection::regular(format!(
                " ({}-{})",
                sdk_artifact.os, sdk_artifact.arch
            )),
        ]);

        let sdk_layer = layers::sdk::handle(&context, sdk_artifact)?;
        let nuget_cache_layer = layers::nuget_cache::handle(&context)?;

        print_section("Publish solution");
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

        print_subsection(vec![
            BuildpackOutputTextSection::regular("Using "),
            BuildpackOutputTextSection::value(build_configuration.clone()),
            BuildpackOutputTextSection::regular(" build configuration"),
        ]);

        let mut publish_command = Command::from(DotnetPublishCommand {
            path: solution.path.clone(),
            configuration: buildpack_configuration.build_configuration,
            runtime_identifier: runtime_identifier::get_runtime_identifier(target_os, target_arch),
            verbosity_level: buildpack_configuration.msbuild_verbosity_level,
        });
        publish_command
            .current_dir(&context.app_dir)
            .envs(&command_env.apply(Scope::Build, &Env::from_current()));

        print_subsection(vec![
            BuildpackOutputTextSection::regular("Running "),
            BuildpackOutputTextSection::Command(command_to_string(&publish_command)),
        ]);
        track_timing(|| {
            run_command(
                publish_command,
                false,
                DotnetBuildpackError::PublishCommandIoError,
                DotnetBuildpackError::PublishCommandNonZeroExitCode,
            )
        })?;

        layers::runtime::handle(&context, &sdk_layer.path())?;

        print_section("Setting launch table");
        print_subsection("Detecting process types from published artifacts");
        let mut launch_builder = LaunchBuilder::new();
        match launch_process::detect_solution_processes(&solution) {
            Ok(processes) => {
                if processes.is_empty() {
                    print_subsection("No processes were detected");
                }
                for process in processes {
                    print_subsection(vec![
                        BuildpackOutputTextSection::regular("Added "),
                        BuildpackOutputTextSection::value(process.r#type.to_string()),
                        BuildpackOutputTextSection::regular(format!(
                            ": {}",
                            process.command.join(" ")
                        )),
                    ]);
                    launch_builder.process(process);
                }
            }
            Err(error) => log_launch_process_detection_warning(error),
        };

        BuildResultBuilder::new()
            .launch(launch_builder.build())
            .build()
    }

    fn on_error(&self, error: libcnb::Error<Self::Error>) {
        errors::on_error(error);
    }
}

fn command_to_string(cmd: &Command) -> String {
    shell_words::join(
        std::iter::once(cmd.get_program())
            .chain(cmd.get_args())
            .map(OsStr::to_string_lossy),
    )
}

fn get_solution_to_publish(app_dir: &Path) -> Result<Solution, DotnetBuildpackError> {
    let solution_file_paths =
        detect::solution_file_paths(app_dir).expect("function to pass after detection");
    // TODO: Handle situation where multiple solution files are found (e.g. by logging a
    // warning, or by building all solutions).
    if let Some(solution_file) = solution_file_paths.first() {
        Solution::load_from_path(solution_file).map_err(DotnetBuildpackError::LoadSolutionFile)
    } else {
        let project_file_paths =
            detect::project_file_paths(app_dir).expect("function to pass after detection");

        match project_file_paths.as_slice() {
            [single_project] => Ok(Solution::ephemeral(
                Project::load_from_path(single_project)
                    .map_err(DotnetBuildpackError::LoadProjectFile)?,
            )),
            _ => Err(DotnetBuildpackError::MultipleRootDirectoryProjectFiles(
                project_file_paths,
            )),
        }
    }
}

fn get_solution_sdk_version_requirement(
    solution: &Solution,
) -> Result<VersionReq, DotnetBuildpackError> {
    solution
        .projects
        .iter()
        .map(|project| {
            project
                .target_framework
                .parse::<TargetFrameworkMoniker>()
                .map_err(DotnetBuildpackError::ParseTargetFrameworkMoniker)
        })
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        // Select the most recent TFM sorted lexicographically, which is sufficient for now as the
        // only expected TFMs follow a consistent format: `netX.0` (e.g. `net6.0`, `net8.0` etc).
        .max_by_key(|tfm| tfm.version_part.clone())
        .ok_or_else(|| DotnetBuildpackError::NoSolutionProjects(solution.path.clone()))
        .map(|tfm| {
            VersionReq::try_from(tfm).map_err(DotnetBuildpackError::ParseSolutionVersionRequirement)
        })?
}

fn detect_global_json_sdk_version_requirement(
    app_dir: &Path,
) -> Option<Result<VersionReq, DotnetBuildpackError>> {
    detect::global_json_file(app_dir).map(|file| {
        VersionReq::try_from(
            fs::read_to_string(file.as_path())
                .map_err(DotnetBuildpackError::ReadGlobalJsonFile)?
                .parse::<GlobalJson>()
                .map_err(DotnetBuildpackError::ParseGlobalJson)?,
        )
        .map_err(DotnetBuildpackError::ParseGlobalJsonVersionRequirement)
    })
}

fn log_launch_process_detection_warning(error: LaunchProcessDetectionError) {
    match error {
        LaunchProcessDetectionError::ProcessType(process_type_error) => print_warning(
            "Launch process detection error",
            formatdoc! {"
                {process_type_error}

                We detected an invalid launch process type.

                The buildpack automatically tries to register Cloud Native Buildpacks (CNB)
                process types for console and web projects after successfully publishing an
                application.

                Process type names are based on the filenames of compiled project executables,
                which is usually the project name. For example, `webapi` for a `webapi.csproj`
                project. In some cases, these names are be incompatible with the CNB spec as 
                process types can only contain numbers, letters, and the characters `.`, `_`,
                and `-`.

                To use this automatic launch process type registration, see the warning details
                above to troubleshoot and make necessary adjustments.

                If you think you found a bug in the buildpack, or have feedback on improving
                the behavior for your use case, file an issue here:
                https://github.com/heroku/buildpacks-dotnet/issues/new
            "},
        ),
    }
}

#[derive(Debug)]
enum DotnetBuildpackError {
    BuildpackDetection(io::Error),
    NoSolutionProjects(PathBuf),
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
    PublishCommandIoError(io::Error),
    PublishCommandNonZeroExitCode(Output),
    CopyRuntimeFiles(io::Error),
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
