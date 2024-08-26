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
use bullet_stream::state::Bullet;
use bullet_stream::{style, Print};
use fun_run::CommandWithName;
use indoc::formatdoc;
use inventory::artifact::{Arch, Os};
use inventory::inventory::{Inventory, ParseInventoryError};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::LaunchBuilder;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer_env::Scope;
use libcnb::{buildpack_main, Buildpack, Env};
use semver::{Version, VersionReq};
use sha2::Sha512;
use std::io::Stdout;
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
            Print::new(std::io::stdout())
                .without_header()
                .warning(
                    "No .NET solution or project files (such as `foo.sln` or `foo.csproj`) found.",
                )
                .done();
            DetectResultBuilder::fail().build()
        }
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        let buildpack_configuration = DotnetBuildpackConfiguration::try_from(&Env::from_current())
            .map_err(DotnetBuildpackError::ParseBuildpackConfiguration)?;

        let mut log = Print::new(std::io::stdout()).h2("Heroku .NET Buildpack");
        let mut log_bullet = log.bullet("SDK version detection");

        let solution = get_solution_to_publish(&context.app_dir)?;

        log_bullet = log_bullet.sub_bullet(format!(
            "Detected .NET file to publish: {}",
            style::value(solution.path.to_string_lossy())
        ));

        let sdk_version_requirement = if let Some(version_req) =
            detect_global_json_sdk_version_requirement(&context.app_dir)
        {
            log_bullet =
                log_bullet.sub_bullet("Detecting version requirement from root global.json file");
            version_req?
        } else {
            log_bullet = log_bullet.sub_bullet(format!(
                "Inferring version requirement from {}",
                style::value(solution.path.to_string_lossy())
            ));
            get_solution_sdk_version_requirement(&solution)?
        };

        log_bullet = log_bullet.sub_bullet(format!(
            "Detected version requirement: {}",
            style::value(sdk_version_requirement.to_string())
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

        log = log_bullet
            .sub_bullet(format!(
                "Resolved .NET SDK version {} {}",
                style::value(sdk_artifact.version.to_string()),
                style::details(format!("{}-{}", sdk_artifact.os, sdk_artifact.arch))
            ))
            .done();

        let (sdk_layer, log) = layers::sdk::handle(&context, log, sdk_artifact)?;
        let (nuget_cache_layer, mut log) = layers::nuget_cache::handle(&context, log)?;

        log_bullet = log.bullet("Publish solution");
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
        log_bullet = log_bullet.sub_bullet(format!(
            "Using {} build configuration",
            style::value(build_configuration.clone())
        ));

        let runtime_identifier = runtime_identifier::get_runtime_identifier(target_os, target_arch);
        let launch_processes_result = launch_process::detect_solution_processes(
            &solution,
            &build_configuration,
            &runtime_identifier,
        );

        let mut publish_command = Command::from(DotnetPublishCommand {
            path: solution.path,
            configuration: buildpack_configuration.build_configuration,
            runtime_identifier: runtime_identifier.clone(),
            verbosity_level: buildpack_configuration.msbuild_verbosity_level,
        });
        publish_command
            .current_dir(&context.app_dir)
            .envs(&command_env.apply(Scope::Build, &Env::from_current()));

        log_bullet
            .stream_with(
                format!("Running {}", style::command(publish_command.name())),
                |stdout, stderr| publish_command.stream_output(stdout, stderr),
            )
            .map_err(DotnetBuildpackError::PublishCommand)?;
        log = log_bullet.done();

        layers::runtime::handle(&context, &sdk_layer.path(), &runtime_identifier)?;

        let mut build_result_builder = BuildResultBuilder::new();
        match launch_processes_result {
            Ok(processes) => {
                // TODO: Print log information about registered processes
                build_result_builder =
                    build_result_builder.launch(LaunchBuilder::new().processes(processes).build());
            }
            Err(error) => log = log_launch_process_detection_warning(error, log),
        }
        log.done();
        build_result_builder.build()
    }

    fn on_error(&self, error: libcnb::Error<Self::Error>) {
        errors::on_error(error);
    }
}

fn get_solution_to_publish(app_dir: &Path) -> Result<Solution, DotnetBuildpackError> {
    let solution_file_paths =
        detect::solution_file_paths(app_dir).expect("function to pass after detection");
    // TODO: Handle situation where multiple solution files are found (e.g. by logging a
    // warning, or by building all solutions).
    if let Some(solution_file) = solution_file_paths.first() {
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
            .ok_or(DotnetBuildpackError::NoSolutionProjects(
                solution.path.clone(),
            ))?,
    )
    .map_err(DotnetBuildpackError::ParseSolutionVersionRequirement)
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

fn log_launch_process_detection_warning(
    error: LaunchProcessDetectionError,
    log: Print<Bullet<Stdout>>,
) -> Print<Bullet<Stdout>> {
    match error {
        LaunchProcessDetectionError::ProcessType(process_type_error) => log.warning(formatdoc! {"
            {process_type_error}

            Launch process detection error

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
        "}),
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
    PublishCommand(fun_run::CmdError),
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
