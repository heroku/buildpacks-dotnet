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
        DotnetBuildpackError::NoDotnetFiles => log_error(
            "No .NET solution or project files found",
            formatdoc! {"
                While determining the .NET file to publish, neither a solution or project file was found.
                This should never occur, as the detect phase should only succeed if a publishable .NET file
                was found.

                If you see this error, please file an issue:
                https://github.com/heroku/buildpacks-dotnet/issues/new
            "},
        ),
        DotnetBuildpackError::NoSolutionProjects => {
            log_error("No project references found in solution", String::new());
        }
        DotnetBuildpackError::MultipleProjectFiles(message) => log_error(
            "Multiple .NET project files",
            formatdoc! {"
                The root directory contains multiple .NET project files: {message}"
            },
        ),
        DotnetBuildpackError::LoadDotnetSolutionFile(error) => match error {
            solution::LoadError::ReadSolutionFile(io_error) => log_io_error(
                "Unable to load solution file",
                "reading solution file",
                io_error,
            ),
            solution::LoadError::LoadProject(load_project_error) => {
                on_load_dotnet_project_error(load_project_error, "reading solution project files");
            }
        },
        DotnetBuildpackError::LoadDotnetProjectFile(error) => {
            on_load_dotnet_project_error(error, "reading root project file");
        }
        // TODO: Add the erroneous input values to these error messages
        DotnetBuildpackError::ParseTargetFrameworkMoniker(error) => match error {
            ParseTargetFrameworkError::InvalidFormat => {
                log_error("Invalid target framework moniker format", String::new());
            }
            ParseTargetFrameworkError::UnsupportedOSTfm => {
                log_error(
                    "Unsupported OS-specific target framework moniker",
                    String::new(),
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
                The root directory `global.json` file could not be parsed.

                Details: {error}
            "},
        ),
        DotnetBuildpackError::ParseGlobalJsonVersionRequirement(error) => log_error(
            "Error parsing global.json version requirement",
            formatdoc! {"
                The version requirement could not be parsed.
                
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
        DotnetBuildpackError::ParseVersionRequirement(error) => log_error(
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
            "},
        ),
        DotnetBuildpackError::SdkLayer(error) => match error {
            SdkLayerError::DownloadSdk(error) => log_error(
                "Couldn't download .NET SDK",
                formatdoc! {"
                    Details: {error}
                "},
            ),
            SdkLayerError::ReadSdkArchive(io_error) => {
                log_io_error(
                    "Couldn't read .NET SDK archive",
                    "reading downloaded .NET SDK archive",
                    io_error,
                );
            }
            SdkLayerError::VerifyChecksum { expected, actual } => log_error(
                "Corrupted .NET SDK download",
                formatdoc! {"
                    Validation of the downloaded .NET SDK failed due to a checksum mismatch.

                    Expected: {expected}
                    Actual: {actual}
                ", expected = hex::encode(expected), actual = hex::encode(actual) },
            ),
            SdkLayerError::OpenSdkArchive(io_error) => {
                log_io_error(
                    "Couldn't open .NET SDK archive",
                    "opening downloaded .NET SDK archive",
                    io_error,
                );
            }
            SdkLayerError::UntarSdk(io_error) => log_io_error(
                "Couldn't decompress .NET SDK",
                "untarring .NET SDK archive",
                io_error,
            ),
        },
        DotnetBuildpackError::ParseDotnetBuildpackConfiguration(error) => match error {
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
        DotnetBuildpackError::CopyRuntimeFilesToRuntimeLayer(io_error) => log_io_error(
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
