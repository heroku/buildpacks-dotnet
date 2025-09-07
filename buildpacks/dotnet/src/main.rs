mod detect;
mod dotnet;
mod dotnet_buildpack_configuration;
mod dotnet_layer_env;
mod dotnet_sdk_command;
mod errors;
mod launch_process;
mod layers;
mod project_toml;
mod utils;

use crate::dotnet::global_json::{GlobalJson, SdkConfig};
use crate::dotnet::project::Project;
use crate::dotnet::runtime_identifier;
use crate::dotnet::solution::Solution;
use crate::dotnet::target_framework_moniker::{ParseTargetFrameworkError, TargetFrameworkMoniker};
use crate::dotnet_buildpack_configuration::{
    DotnetBuildpackConfiguration, DotnetBuildpackConfigurationError, ExecutionEnvironment,
};
use crate::dotnet_sdk_command::{DotnetPublishCommand, DotnetTestCommand};
use crate::layers::sdk::SdkLayerError;
use crate::project_toml::DotnetConfig;
use bullet_stream::fun_run::{self, CommandWithName};
use bullet_stream::global::print;
use bullet_stream::style;
use inventory::artifact::{Arch, Os};
use inventory::{Inventory, ParseInventoryError};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::launch::{LaunchBuilder, Process};
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer::UncachedLayerDefinition;
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::{Buildpack, Env, Target, buildpack_main};
use libherokubuildpack::inventory;
use libherokubuildpack::inventory::artifact::Artifact;
use semver::{Version, VersionReq};
use sha2::Sha512;
use std::io;
use std::io::{Write, stderr};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::instrument;

struct DotnetBuildpack;

impl Buildpack for DotnetBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = DotnetBuildpackError;

    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        // Check if a solution file is configured in project.toml
        let project_toml_config = load_project_toml_config(&context.app_dir)?;
        if let Some(config) = &project_toml_config
            && config.solution_file.is_some()
        {
            return DetectResultBuilder::pass().build();
        }

        // Otherwise, perform standard detection by looking for .NET files
        let paths = detect::get_files_with_extensions(
            &context.app_dir,
            &["sln", "csproj", "vbproj", "fsproj"],
        )
        .map_err(DotnetBuildpackError::BuildpackDetection)?;

        if paths.is_empty() {
            println!("No .NET application found. This buildpack requires either:");
            println!(
                "- .NET solution (`.sln`) or project (`.csproj`, `.vbproj`, `.fsproj`) files in the root directory"
            );
            println!("- A `solution_file` configured in `project.toml`");
            let _ = std::io::stdout().flush();
            DetectResultBuilder::fail().build()
        } else {
            DetectResultBuilder::pass().build()
        }
    }

    #[allow(clippy::too_many_lines)]
    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        let project_toml_config = load_project_toml_config(&context.app_dir)?;

        let buildpack_configuration = DotnetBuildpackConfiguration::try_from_env_and_project_toml(
            &Env::from_current(),
            project_toml_config.as_ref(),
        )
        .map_err(DotnetBuildpackError::ParseBuildpackConfiguration)?;

        bullet_stream::global::set_writer(std::io::stdout());
        print::h2("Heroku .NET Buildpack");
        let started = std::time::Instant::now();
        print::bullet("SDK version detection");

        let solution = if let Some(path) = buildpack_configuration.solution_file {
            print::sub_bullet(format!(
                "Using configured solution file: {}",
                style::value(path.to_string_lossy())
            ));
            Solution::load_from_path(&context.app_dir.join(path))
                .map_err(DotnetBuildpackError::LoadSolutionFile)
        } else {
            get_solution_to_publish(&context.app_dir)
        }?;

        let sdk_version_requirement = detect_sdk_version_requirement(&context, &solution)?;

        let sdk_artifact = resolve_sdk_artifact(&context.target, sdk_version_requirement)?;

        let sdk_scope = match buildpack_configuration.execution_environment {
            ExecutionEnvironment::Production => Scope::Build,
            ExecutionEnvironment::Test => Scope::All,
        };
        let sdk_available_at_launch = matches!(sdk_scope, Scope::Launch | Scope::All);

        let sdk_layer = layers::sdk::handle(&context, sdk_available_at_launch, &sdk_artifact)?;
        sdk_layer.write_env(dotnet_layer_env::generate_layer_env(
            sdk_layer.path().as_path(),
            &sdk_scope,
        ))?;

        let nuget_cache_layer = layers::nuget_cache::handle(&context, sdk_available_at_launch)?;
        nuget_cache_layer.write_env(
            LayerEnv::new()
                .chainable_insert(
                    sdk_scope.clone(),
                    libcnb::layer_env::ModificationBehavior::Override,
                    "NUGET_PACKAGES",
                    nuget_cache_layer.path(),
                )
                .chainable_insert(
                    sdk_scope.clone(),
                    libcnb::layer_env::ModificationBehavior::Default,
                    "NUGET_XMLDOC_MODE",
                    "skip",
                ),
        )?;

        let dotnet_cli_layer = context.uncached_layer(
            layer_name!("dotnet-cli"),
            UncachedLayerDefinition {
                build: true,
                launch: sdk_available_at_launch,
            },
        )?;
        dotnet_cli_layer.write_env(LayerEnv::new().chainable_insert(
            sdk_scope.clone(),
            libcnb::layer_env::ModificationBehavior::Override,
            "DOTNET_CLI_HOME",
            dotnet_cli_layer.path(),
        ))?;

        let command_env = dotnet_cli_layer.read_env()?.apply(
            Scope::Build,
            &nuget_cache_layer.read_env()?.apply(
                Scope::Build,
                &sdk_layer
                    .read_env()?
                    .apply(Scope::Build, &Env::from_current()),
            ),
        );

        if let Some(manifest_path) = detect::dotnet_tools_manifest_file(&context.app_dir) {
            let mut restore_tools_command = Command::new("dotnet");
            restore_tools_command
                .args([
                    "tool",
                    "restore",
                    "--tool-manifest",
                    &manifest_path.to_string_lossy(),
                ])
                .current_dir(&context.app_dir)
                .envs(&command_env);

            print::bullet("Restore .NET tools");
            print::sub_bullet("Tool manifest file detected");
            print::sub_stream_with(
                format!("Running {}", style::command(restore_tools_command.name())),
                |stdout, stderr| restore_tools_command.stream_output(stdout, stderr),
            )
            .map_err(DotnetBuildpackError::RestoreDotnetToolsCommand)?;
        }

        let mut launch_builder = LaunchBuilder::new();
        match buildpack_configuration.execution_environment {
            ExecutionEnvironment::Production => {
                print::bullet("Publish app");

                let mut publish_command = Command::from(DotnetPublishCommand {
                    path: solution.path.clone(),
                    configuration: buildpack_configuration.build_configuration,
                    runtime_identifier: runtime_identifier::get_runtime_identifier(
                        sdk_artifact.os,
                        sdk_artifact.arch,
                    ),
                    verbosity_level: buildpack_configuration.msbuild_verbosity_level,
                });
                publish_command
                    .current_dir(&context.app_dir)
                    .envs(&command_env);

                print::sub_stream_with(
                    format!("Running {}", style::command(publish_command.name())),
                    |stdout, stderr| publish_command.stream_output(stdout, stderr),
                )
                .map_err(DotnetBuildpackError::PublishCommand)?;
                if !sdk_available_at_launch {
                    layers::runtime::handle(&context, &sdk_layer.path())?;
                }

                print::bullet("Process types");
                print::sub_bullet("Detecting process types from published artifacts");
                let processes =
                    launch_process::detect_solution_processes(&context.app_dir, &solution);
                if processes.is_empty() {
                    print::sub_bullet("No processes were detected");
                } else {
                    for process in &processes {
                        print::sub_bullet(format!(
                            "Found {}: {}",
                            style::value(process.r#type.to_string()),
                            process.command.join(" ")
                        ));
                    }
                    if Path::exists(&context.app_dir.join("Procfile")) {
                        print::sub_bullet("Procfile detected");
                        print::sub_bullet(
                            "Skipping process type registration (add process types to your Procfile as needed)",
                        );
                    } else {
                        launch_builder.processes(processes.clone());
                        print::sub_bullet("No Procfile detected");
                        print::sub_bullet("Registering detected process types as launch processes");
                    }
                }
            }
            ExecutionEnvironment::Test => {
                launch_builder.process(Process::from(DotnetTestCommand {
                    path: solution.path,
                    configuration: buildpack_configuration.build_configuration,
                    verbosity_level: buildpack_configuration.msbuild_verbosity_level,
                }));
            }
        }

        print::all_done(&Some(started));

        BuildResultBuilder::new()
            .launch(launch_builder.build())
            .build()
    }

    fn on_error(&self, error: libcnb::Error<Self::Error>) {
        errors::on_error_with_writer(error, stderr());
    }
}

fn load_project_toml_config(app_dir: &Path) -> Result<Option<DotnetConfig>, DotnetBuildpackError> {
    detect::project_toml_file(app_dir).map_or_else(
        || Ok(None),
        |file| {
            fs_err::read_to_string(file)
                .map_err(DotnetBuildpackError::ReadProjectTomlFile)
                .and_then(|content| {
                    project_toml::parse(&content).map_err(DotnetBuildpackError::ParseProjectToml)
                })
        },
    )
}

#[instrument(skip_all, err(Debug), fields(
    os.type = %target.os,
    host.arch = %target.arch,
    cnb.dotnet.version_requirement = %sdk_version_requirement,
))]
fn resolve_sdk_artifact(
    target: &Target,
    mut sdk_version_requirement: VersionReq,
) -> Result<Artifact<Version, Sha512, Option<()>>, DotnetBuildpackError> {
    // When VersionReq is derived from the TFM for .NET 10.0, which has no
    // stable releases, rewrite the requirement to match preview and release
    // candidate releases.
    // TODO: Remove this logic when stable .NET 10.0 releases are added to the
    // the inventory, and update the .NET 10 SDK resolution integration test.
    if sdk_version_requirement
        == VersionReq::try_from(&TargetFrameworkMoniker {
            version_part: String::from("10.0"),
        })
        .expect("TargetFrameworkMoniker should always be parseable")
    {
        sdk_version_requirement = VersionReq::parse("^10.0.100-preview")
            .expect("VersionReq string should always be parseable");
    }

    include_str!("../inventory.toml")
        .parse::<Inventory<_, _, _>>()
        .map_err(DotnetBuildpackError::ParseInventory)
        .and_then(|inventory| {
            inventory
                .resolve(
                    target.os.parse::<Os>().expect("OS should always be parseable, buildpack will not run on unsupported operating systems."),
                    target.arch.parse::<Arch>().expect("Arch should always be parseable, buildpack will not run on unsupported architectures."),
                    &sdk_version_requirement
                )
                .ok_or(DotnetBuildpackError::ResolveSdkVersion(
                    sdk_version_requirement,
                ))
                .cloned()
                .inspect(|artifact|
                    print::sub_bullet(format!(
                        "Resolved .NET SDK version {} {}",
                        style::value(artifact.version.to_string()),
                        style::details(format!("{}-{}", artifact.os, artifact.arch))
                    )))
        })
}

#[instrument(skip_all, err(Debug))]
fn detect_sdk_version_requirement(
    context: &BuildContext<DotnetBuildpack>,
    solution: &Solution,
) -> Result<VersionReq, DotnetBuildpackError> {
    detect_global_json_sdk_configuration(&context.app_dir)?
        .map_or_else(
            || {
                print::sub_bullet(format!(
                    "Inferring version requirement from {}",
                    style::value(solution.path.to_string_lossy())
                ));
                get_solution_sdk_version_requirement(solution)
            },
            |sdk_config| {
                print::sub_bullet("Detecting version requirement from root global.json file");
                VersionReq::try_from(sdk_config)
                    .map_err(DotnetBuildpackError::ParseGlobalJsonVersionRequirement)
            },
        )
        .inspect(|version_req| {
            print::sub_bullet(format!(
                "Detected version requirement: {}",
                style::value(version_req.to_string())
            ));
        })
}

fn get_solution_to_publish(app_dir: &Path) -> Result<Solution, DotnetBuildpackError> {
    let solution_file_paths =
        detect::solution_file_paths(app_dir).expect("function to pass after detection");
    match solution_file_paths.as_slice() {
        // A single solution file was found.
        [solution_file] => {
            print::sub_bullet(format!(
                "Detected .NET solution: {}",
                style::value(solution_file.to_string_lossy())
            ));
            Solution::load_from_path(solution_file).map_err(DotnetBuildpackError::LoadSolutionFile)
        }
        // No solution files were found, so we look for project files.
        [] => {
            let project_file_paths =
                detect::project_file_paths(app_dir).expect("function to pass after detection");

            match project_file_paths.as_slice() {
                [single_project] => Ok(Solution::ephemeral(
                    Project::load_from_path(single_project)
                        .map_err(DotnetBuildpackError::LoadProjectFile)
                        .inspect(|project| {
                            print::sub_bullet(format!(
                                "Detected .NET project: {}",
                                style::value(project.path.to_string_lossy())
                            ));
                        })?,
                )),
                _ => Err(DotnetBuildpackError::MultipleRootDirectoryProjectFiles(
                    project_file_paths,
                )),
            }
        }
        // Multiple solution files were found, so we return an error.
        _ => Err(DotnetBuildpackError::MultipleRootDirectorySolutionFiles(
            solution_file_paths,
        )),
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

fn detect_global_json_sdk_configuration(
    app_dir: &Path,
) -> Result<Option<SdkConfig>, DotnetBuildpackError> {
    detect::global_json_file(app_dir).map_or_else(
        || Ok(None),
        |file| {
            fs_err::read_to_string(file)
                .map_err(DotnetBuildpackError::ReadGlobalJsonFile)
                .and_then(|content| {
                    content
                        .parse::<GlobalJson>()
                        .map_err(DotnetBuildpackError::ParseGlobalJson)
                        .map(|global_json| global_json.sdk)
                })
        },
    )
}

#[derive(Debug)]
enum DotnetBuildpackError {
    BuildpackDetection(io::Error),
    ReadProjectTomlFile(io::Error),
    ParseProjectToml(toml::de::Error),
    NoSolutionProjects(PathBuf),
    MultipleRootDirectorySolutionFiles(Vec<PathBuf>),
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
    RestoreDotnetToolsCommand(fun_run::CmdError),
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
