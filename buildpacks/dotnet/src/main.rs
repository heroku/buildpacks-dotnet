mod detect;
mod dotnet_executable_finder;
mod dotnet_project;
mod dotnet_rid;
mod global_json;
mod layers;
mod tfm;
mod utils;

use crate::dotnet_project::DotnetProject;
use crate::layers::sdk::SdkLayerError;
use crate::utils::StreamedCommandError;
use libcnb::build::BuildResultBuilder;
use libcnb::data::launch::{LaunchBuilder, ProcessBuilder};
use libcnb::data::{layer_name, process_type};
use libcnb::detect::DetectResultBuilder;
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer::{CachedLayerDefinition, InspectExistingAction, InvalidMetadataAction};
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::{buildpack_main, Buildpack, Env};
use libherokubuildpack::log::{log_header, log_info};
use serde::{Deserialize, Serialize};
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

    fn build(
        &self,
        context: libcnb::build::BuildContext<Self>,
    ) -> libcnb::Result<libcnb::build::BuildResult, Self::Error> {
        // TODO: WIP
        let project_files_result = detect::dotnet_project_files(context.app_dir.clone())
            .expect("no issues finding project files after detect");

        let project_file = project_files_result
            .first()
            .expect("at least one project file");

        let dotnet_project = fs::read_to_string(project_file)
            .map_err(SdkLayerError::ReadProjectFile)?
            .parse::<DotnetProject>()
            .map_err(SdkLayerError::ParseDotnetProjectFile)?;

        let runtime_identifier = dotnet_rid::get_dotnet_rid();

        let executable_process = dotnet_executable_finder::determine_executable_path(
            &dotnet_project,
            project_file,
            "Release",
            &runtime_identifier,
        )
        .expect("project to produce an executable");

        log_header(".NET SDK");
        let sdk_layer = layers::sdk::handle(&context)?;

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
                    "--verbosity",
                    "normal",
                    "--configuration",
                    "Release",
                    "--runtime",
                    &runtime_identifier.to_string(),
                ])
                .current_dir(&context.app_dir)
                .envs(&command_env.apply(Scope::Build, &Env::from_current())),
        )
        .map_err(DotnetBuildpackError::PublishCommand)?;

        BuildResultBuilder::new()
            .launch(
                LaunchBuilder::new()
                    .process(
                        ProcessBuilder::new(
                            // TODO: Determine whether project is actually a web project.
                            // Consider adding non-web executables to list of processes after
                            // validating the executable name is valid (especially if build multi-process
                            // solution)
                            process_type!("web"),
                            [
                                "bash",
                                "-c",
                                &format!(
                                    "{} --urls http://0.0.0.0:$PORT",
                                    executable_process.to_string_lossy()
                                ),
                            ],
                        )
                        .default(true)
                        .build(),
                    )
                    .build(),
            )
            .build()
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
    #[error(transparent)]
    SdkLayer(#[from] SdkLayerError),
    #[error("Error reading SDK layer environment")]
    ReadSdkLayerEnvironment(io::Error),
    #[error("Error executing publish task")]
    PublishCommand(#[from] StreamedCommandError),
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
