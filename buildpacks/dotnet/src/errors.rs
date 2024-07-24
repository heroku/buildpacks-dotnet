use crate::dotnet::target_framework_moniker::ParseTargetFrameworkError;
use crate::dotnet::{project, solution};
use crate::dotnet_buildpack_configuration::DotnetBuildpackConfigurationError;
use crate::launch_process::LaunchProcessDetectionError;
use crate::layers::sdk::SdkLayerError;
use crate::utils::StreamedCommandError;
use crate::DotnetBuildpackError;
use indoc::formatdoc;
use libherokubuildpack::log::log_error;
use std::io;

pub(crate) fn on_error(error: libcnb::Error<DotnetBuildpackError>) {
    match error {
        libcnb::Error::BuildpackError(buildpack_error) => on_buildpack_error(&buildpack_error),
        libcnb_error => log_error(
            "Internal buildpack error",
            formatdoc! {"
                An unexpected internal error was reported by the framework used by this buildpack.
        
                If you see this error, please file an issue:
                https://github.com/heroku/buildpacks-dotnet/issues/new
        
                Details: {libcnb_error}
            "},
        ),
    }
}

#[allow(clippy::too_many_lines)]
fn on_buildpack_error(error: &DotnetBuildpackError) {
    match error {
        DotnetBuildpackError::BuildpackDetection(io_error) => log_io_error(
            "Unable to complete buildpack detection",
            "determining if the .NET buildpack should be run for this application",
            io_error,
        ),
        DotnetBuildpackError::NoSolutionProjects(solution_path) => {
            log_error(
                "No project references found in solution",
                formatdoc! {"
                The solution file \"{}\" did not reference any projects.

                This buildpack will prefer building a solution file over a project file when both are present in the root directory.
                
                To resolve this issue you may want to either:
                  * Delete the solution file to allow a root project file to be built instead.
                  * Reference projects that should be built from the solution file.
                ", solution_path.to_string_lossy()},
            );
        }
        DotnetBuildpackError::MultipleRootDirectoryProjectFiles(project_file_paths) => log_error(
            "Multiple .NET project files",
            formatdoc! {"
                The root directory contains multiple .NET project files: {}.

                Multiple project files in the root directory is not supported, as this is highly likely to
                produce unexpected results. Reorganizing the directory and project structure to only include
                one project file per folder (not only the root folder) is recommended.

                If you are porting an application from .NET Framework to .NET, or wish to compile both side-by-side,
                see this article for useful project organization advice: https://learn.microsoft.com/en-us/dotnet/core/porting/project-structure
                ", project_file_paths.iter()
                    .map(|f| f.to_string_lossy().to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            },
        ),
        DotnetBuildpackError::LoadSolutionFile(error) => match error {
            solution::LoadError::ReadSolutionFile(io_error) => log_io_error(
                "Unable to load solution file",
                "reading solution file",
                io_error,
            ),
            solution::LoadError::LoadProject(load_project_error) => {
                on_load_dotnet_project_error(load_project_error, "reading solution project files");
            }
        },
        DotnetBuildpackError::LoadProjectFile(error) => {
            on_load_dotnet_project_error(error, "reading root project file");
        }
        DotnetBuildpackError::ParseTargetFrameworkMoniker(error) => match error {
            ParseTargetFrameworkError::InvalidFormat(tfm)
            | ParseTargetFrameworkError::UnsupportedOSTfm(tfm) => {
                log_error(
                    "Invalid target framework",
                    formatdoc! {"
                        The detected target framework moniker `{tfm}` is either invalid or unsupported. This buildpack
                        currently supports the following TFMs: `net5.0`, `net6.0`, `net7.0` and `.net8.0`.

                        For more information, see: https://learn.microsoft.com/en-us/dotnet/standard/frameworks#latest-versions
                    "},
                );
            }
        },
        DotnetBuildpackError::ReadGlobalJsonFile(error) => log_io_error(
            "Error reading global.json file",
            "detecting SDK version requirement",
            error,
        ),
        DotnetBuildpackError::ParseGlobalJson(error) => log_error(
            "Invalid global.json format",
            formatdoc! {"
                The root directory `global.json` file contents could not be parsed.

                Details: {error}
            "},
        ),
        DotnetBuildpackError::ParseGlobalJsonVersionRequirement(error) => log_error(
            "Error parsing global.json version requirement",
            formatdoc! {"
                The .NET SDK version requirement could not be parsed.
                
                Details: {error}
            "},
        ),
        DotnetBuildpackError::ParseInventory(error) => log_error(
            "Invalid inventory.toml file",
            formatdoc! {"
                The inventory of .NET SDK releases could not be parsed. This error should
                never occur to users of this buildpack and is almost always a buildpack bug.

                If you see this error, please file an issue:
                https://github.com/heroku/buildpacks-dotnet/issues/new

                Details: {error}
            "},
        ),
        DotnetBuildpackError::ParseSolutionVersionRequirement(error) => log_error(
            "Invalid .NET SDK version requirement",
            formatdoc! {"
                The inferred .NET SDK version requirement could not be parsed.
    
                Details: {error}
            "},
        ),
        DotnetBuildpackError::ResolveSdkVersion(version_req) => log_error(
            "Unsupported .NET SDK version",
            formatdoc! {"
                A compatible .NET SDK release could not be resolved from the detected version requirement ({version_req}).

                For a complete inventory of supported .NET SDK versions and platforms, see: https://github.com/heroku/buildpacks-dotnet/blob/main/buildpacks/dotnet/inventory.toml.
            "},
        ),
        DotnetBuildpackError::SdkLayer(error) => match error {
            SdkLayerError::DownloadArchive(error) => log_error(
                "Failed to download .NET SDK",
                formatdoc! {"
                    An unexpected error occurred while downloading the .NET SDK. This error can occur due to an unstable network connection. 

                    Use the error details below to troubleshoot and retry your build. 

                    If the issue persists and you think you found a bug in the buildpack, reproduce the issue locally with a minimal example and file an issue here:
                    https://github.com/heroku/buildpacks-dotnet/issues/new

                    Details: {error}
                "},
            ),
            SdkLayerError::ReadArchive(io_error) => {
                log_io_error(
                    "Couldn't read .NET SDK archive",
                    "reading downloaded .NET SDK archive",
                    io_error,
                );
            }
            SdkLayerError::VerifyArchiveChecksum { expected, actual } => log_error(
                "Corrupted .NET SDK download",
                formatdoc! {"
                    Validation of the downloaded .NET SDK failed due to a checksum mismatch.

                    Expected: {expected}
                    Actual: {actual}
                ", expected = hex::encode(expected), actual = hex::encode(actual) },
            ),
            SdkLayerError::OpenArchive(io_error) => {
                log_io_error(
                    "Couldn't open .NET SDK archive",
                    "opening downloaded .NET SDK archive",
                    io_error,
                );
            }
            SdkLayerError::DecompressArchive(io_error) => log_io_error(
                "Couldn't decompress .NET SDK",
                "untarring .NET SDK archive",
                io_error,
            ),
        },
        DotnetBuildpackError::ParseBuildpackConfiguration(error) => match error {
            DotnetBuildpackConfigurationError::InvalidMsbuildVerbosityLevel(verbosity_level) => {
                log_error(
                    "Invalid MSBuild verbosity level",
                    formatdoc! {"
                        The 'MSBUILD_VERBOSITY_LEVEL' environment variable value ('{verbosity_level}') could not be parsed. Did you mean one of the following?

                        d
                        detailed
                        diag
                        diagnostic
                        m
                        minimal
                        n
                        normal
                        q
                        quiet
                    "},
                );
            }
        },
        DotnetBuildpackError::PublishCommand(error) => match error {
            StreamedCommandError::Io(io_error) => log_io_error(
                "Unable to publish .NET file",
                "running the command to publish .NET file",
                io_error,
            ),
            StreamedCommandError::NonZeroExitStatus(exit_status) => log_error(
                "Unable to publish .NET file",
                formatdoc! {"
                    The 'dotnet publish' command did not exit successfully ({exit_status}).
                    
                    See the log output above for more information.
                    
                    In some cases, this happens due to an unstable network connection.
                    Please try again to see if the error resolves itself.
                "},
            ),
        },
        DotnetBuildpackError::CopyRuntimeFiles(io_error) => log_io_error(
            "Error copying .NET runtime files",
            "copying .NET runtime files from the sdk layer to the runtime layer",
            io_error,
        ),
        DotnetBuildpackError::LaunchProcessDetection(error) => match error {
            LaunchProcessDetectionError::ProcessType(process_type_error) => {
                log_error(
                    "Launch process detection error",
                    formatdoc! {"
                        An invalid launch process type was detected.
    
                        Details: {process_type_error}
                    "},
                );
            }
        },
    }
}

fn on_load_dotnet_project_error(error: &project::LoadError, occurred_while: &str) {
    match error {
        project::LoadError::ReadProjectFile(io_error) => {
            log_io_error("Unable to read project", occurred_while, io_error);
        }
        project::LoadError::XmlParseError(error) => log_error(
            "Unable to parse project file",
            formatdoc! {"
                The project file XML content could not be parsed. This usually indicates an error in the project file.
                    
                Details: {error}"},
        ),
        project::LoadError::MissingTargetFramework => {
            log_error("Project file is missing TargetFramework", String::new());
        }
    }
}

fn log_io_error(header: &str, occurred_while: &str, io_error: &io::Error) {
    log_error(
        header,
        formatdoc! {"
            An unexpected I/O error occurred while {occurred_while}.
            
            Details: {io_error}
        "},
    );
}
