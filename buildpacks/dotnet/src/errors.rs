use crate::DotnetBuildpackError;
use crate::dotnet::target_framework_moniker::ParseTargetFrameworkError;
use crate::dotnet::{project, solution};
use crate::dotnet_buildpack_configuration::{
    DotnetBuildpackConfigurationError, ExecutionEnvironmentError, ParseVerbosityLevelError,
};
use crate::layers::sdk::SdkLayerError;
use bullet_stream::{Print, fun_run, style};
use indoc::formatdoc;
use std::io::{self, Write};

pub(crate) fn on_error_with_writer(
    error: libcnb::Error<DotnetBuildpackError>,
    mut writer: impl Write,
) {
    match error {
        libcnb::Error::BuildpackError(buildpack_error) => {
            on_buildpack_error_with_writer(&buildpack_error, writer);
        }
        libcnb_error => log_error_to(
            &mut writer,
            "Heroku .NET Buildpack internal buildpack error",
            formatdoc! {"
                The framework used by this buildpack encountered an unexpected error.

                If you can’t deploy to Heroku due to this issue, check the official Heroku Status page at
                status.heroku.com for any ongoing incidents. After all incidents resolve, retry your build.

                Use the debug information above to troubleshoot and retry your build. If you think you found a
                bug in the buildpack, reproduce the issue locally with a minimal example and file an issue here:
                https://github.com/heroku/buildpacks-dotnet/issues/new
            "},
            Some(libcnb_error.to_string()),
        ),
    }
}

#[allow(clippy::too_many_lines)]
fn on_buildpack_error_with_writer(error: &DotnetBuildpackError, mut writer: impl Write) {
    match error {
        DotnetBuildpackError::BuildpackDetection(io_error) => log_io_error_to(
            writer,
            "Error completing buildpack detection",
            "determining if we must run the Heroku .NET buildpack for this application",
            io_error,
        ),
        DotnetBuildpackError::ReadProjectTomlFile(io_error) => log_io_error_to(
            &mut writer,
            "Error reading `project.toml` file",
            "reading `project.toml` file",
            io_error,
        ),
        DotnetBuildpackError::ParseProjectToml(error) => log_error_to(
            &mut writer,
            "Invalid `project.toml` file",
            formatdoc! {"
                We can't parse the `project.toml` file because it contains invalid TOML.

                Use the debug information above to troubleshoot and retry your build.
            "},
            Some(error.to_string()),
        ),
        DotnetBuildpackError::NoSolutionProjects(solution_path) => {
            log_error_to(
                &mut writer,
                "No project references found in solution",
                formatdoc! {"
                The solution file `{}` has no project references.

                This buildpack prefers building a solution file over a project file if both
                are present in the root directory.

                To resolve this issue,
                * Delete the solution file to build a root project file instead.
                * Or reference the projects to build from the solution file.

                ", solution_path.to_string_lossy()},
                None,
            );
        }
        DotnetBuildpackError::MultipleRootDirectorySolutionFiles(paths) => log_error_to(
            &mut writer,
            "Multiple .NET solution files",
            formatdoc! {"
                The root directory contains multiple .NET solution files: `{}`.

                When there are multiple solution files in the root directory, you must specify
                which one to use.

                For more information, see:
                https://github.com/heroku/buildpacks-dotnet#solution-file
                ", paths.iter()
                    .map(|f| f.to_string_lossy().to_string())
                    .collect::<Vec<String>>()
                    .join("`, `"),
            },
            None,
        ),
        DotnetBuildpackError::MultipleRootDirectoryProjectFiles(paths) => log_error_to(
            &mut writer,
            "Multiple .NET project files",
            formatdoc! {"
                The root directory contains multiple .NET project files: `{}`.

                We don’t support having multiple project files in the root directory to prevent
                unexpected results. We recommend reorganizing the directory and project
                structure to include only one project file per folder.

                If you’re porting an application from .NET Framework to .NET, or compiling both
                side-by-side, see Microsoft’s documentation for project organization guidance:
                https://learn.microsoft.com/en-us/dotnet/core/porting/project-structure
                ", paths.iter()
                    .map(|f| f.to_string_lossy().to_string())
                    .collect::<Vec<String>>()
                    .join("`, `"),
            },
            None,
        ),
        DotnetBuildpackError::LoadSolutionFile(error) => match error {
            solution::LoadError::ReadSolutionFile(io_error) => log_io_error_to(
                &mut writer,
                "Error loading solution file",
                "reading the solution file",
                io_error,
            ),
            solution::LoadError::LoadProject(load_project_error) => {
                on_load_dotnet_project_error_with_writer(
                    &mut writer,
                    load_project_error,
                    "reading solution project files",
                );
            }
            solution::LoadError::ProjectNotFound(project_path) => {
                log_error_to(
                    &mut writer,
                    "Missing project referenced in solution",
                    formatdoc! {"
                    The solution references a project file that does not exist: `{}`.

                    This error occurs when a project referenced in the solution file cannot be found at
                    the expected location. This can happen if:
                    * The project was moved or renamed.
                    * The project was deleted but not removed from the solution.
                    * The project path in the solution file is incorrect.

                    To resolve this issue:
                    * Verify the project exists at the expected location.
                    * Update the project reference path in the solution file.
                    * Or remove the project reference from the solution if it's no longer needed.
                    ", project_path.to_string_lossy()},
                    None,
                );
            }
            solution::LoadError::SlnxParseError(error) => {
                log_error_to(
                    &mut writer,
                    "Error parsing solution file",
                    formatdoc! {"
                        We can't parse the solution file because it contains invalid XML.

                        Use the debug information above to troubleshoot and retry your build.
                    "},
                    Some(error.to_string()),
                );
            }
        },
        DotnetBuildpackError::LoadProjectFile(error) => {
            on_load_dotnet_project_error_with_writer(
                &mut writer,
                error,
                "reading root project file",
            );
        }
        DotnetBuildpackError::ParseTargetFrameworkMoniker(error) => match error {
            ParseTargetFrameworkError::InvalidFormat(tfm)
            | ParseTargetFrameworkError::UnsupportedOSTfm(tfm) => {
                log_error_to(
                    &mut writer,
                    "Unsupported target framework",
                    formatdoc! {"
                        The detected target framework moniker `{tfm}` is either invalid or unsupported. This
                        buildpack currently supports the following TFMs: `net6.0`, `net7.0`, `net8.0`, `net9.0`.

                        For more information, see:
                        https://github.com/heroku/buildpacks-dotnet#net-version
                    "},
                    None,
                );
            }
        },
        DotnetBuildpackError::ReadGlobalJsonFile(error) => log_io_error_to(
            &mut writer,
            "Error reading `global.json` file",
            "detecting SDK version requirement",
            error,
        ),
        DotnetBuildpackError::ParseGlobalJson(error) => log_error_to(
            &mut writer,
            "Invalid `global.json` file",
            formatdoc! {"
                We can’t parse the root directory `global.json` file because it contains invalid JSON.

                Use the debug information above to troubleshoot and retry your build.
            "},
            Some(error.to_string()),
        ),
        // TODO: Consider adding more specific errors for the parsed values (e.g. an invalid rollForward value)
        DotnetBuildpackError::ParseGlobalJsonVersionRequirement(error) => log_error_to(
            &mut writer,
            "Error parsing `global.json` version requirement",
            formatdoc! {"
                We can’t parse the .NET SDK version requirement.

                Use the debug information above to troubleshoot and retry your build. For more
                information, see:
                https://github.com/heroku/buildpacks-dotnet#net-version
            "},
            Some(error.to_string()),
        ),
        DotnetBuildpackError::ParseInventory(error) => log_error_to(
            &mut writer,
            "Invalid `inventory.toml` file",
            formatdoc! {"
                We can’t parse the inventory of .NET SDK releases. This error
                is almost always a buildpack bug.

                If you see this error, please file an issue here:
                https://github.com/heroku/buildpacks-dotnet/issues/new

            "},
            Some(error.to_string()),
        ),
        DotnetBuildpackError::ParseSolutionVersionRequirement(error) => log_error_to(
            &mut writer,
            "Invalid .NET SDK version requirement",
            formatdoc! {"
                We can’t parse the inferred .NET SDK version requirement.

                Use the debug information above to troubleshoot and retry your build. If you think
                you found a bug in the buildpack, reproduce the issue locally with a minimal
                example and file an issue here:
                https://github.com/heroku/buildpacks-dotnet/issues/new

            "},
            Some(error.to_string()),
        ),
        DotnetBuildpackError::ResolveSdkVersion(version_req) => log_error_to(
            &mut writer,
            "Unsupported .NET SDK version",
            formatdoc! {"
                We can’t find a compatible .NET SDK release for the detected version
                requirement ({version_req}).

                For a complete inventory of supported .NET SDK versions and platforms, see:
                https://github.com/heroku/buildpacks-dotnet/blob/main/buildpacks/dotnet/inventory.toml
            "},
            None,
        ),
        DotnetBuildpackError::SdkLayer(error) => match error {
            SdkLayerError::DownloadArchive(error) => log_error_to(
                &mut writer,
                "Failed to download .NET SDK",
                formatdoc! {"
                    An unexpected error occurred while downloading the .NET SDK. This error can occur
                    due to an unstable network connection.

                    Use the debug information above to troubleshoot and retry your build.
                "},
                Some(error.to_string()),
            ),
            SdkLayerError::ReadArchive(io_error) => {
                log_io_error_to(
                    &mut writer,
                    "Error reading downloaded SDK archive",
                    "calculating checksum for the downloaded .NET SDK archive",
                    io_error,
                );
            }
            SdkLayerError::VerifyArchiveChecksum { expected, actual } => log_error_to(
                &mut writer,
                "Corrupted .NET SDK download",
                formatdoc! {"
                    Validation of the downloaded .NET SDK failed due to a checksum mismatch. This error can
                    occur intermittently.

                    Use the debug information above to troubleshoot and retry your build. If the issue persists,
                    file an issue here:
                    https://github.com/heroku/buildpacks-dotnet/issues/new

                    Expected: {expected}
                    Actual: {actual}
                ", expected = hex::encode(expected), actual = hex::encode(actual) },
                None,
            ),
            SdkLayerError::OpenArchive(io_error) => {
                log_io_error_to(
                    &mut writer,
                    "Error reading downloaded SDK archive",
                    "decompressing downloaded .NET SDK archive",
                    io_error,
                );
            }
            SdkLayerError::DecompressArchive(io_error) => log_io_error_to(
                &mut writer,
                "Failed to decompress .NET SDK",
                "extracting .NET SDK archive contents",
                io_error,
            ),
        },
        DotnetBuildpackError::ParseBuildpackConfiguration(error) => match error {
            DotnetBuildpackConfigurationError::VerbosityLevel(ParseVerbosityLevelError(
                verbosity_level,
            )) => {
                log_error_to(
                    &mut writer,
                    "Invalid MSBuild verbosity level",
                    formatdoc! {"
                        The `MSBUILD_VERBOSITY_LEVEL` environment variable value (`{verbosity_level}`)
                        is invalid. Did you mean one of the following supported values?

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
                    None,
                );
            }
            DotnetBuildpackConfigurationError::ExecutionEnvironment(error) => match error {
                ExecutionEnvironmentError::UnsupportedExecutionEnvironment(
                    execution_environment,
                ) => {
                    log_error_to(
                        &mut writer,
                        "Unsupported execution environment",
                        formatdoc! {"
                            The `CNB_EXEC_ENV` environment variable value (`{execution_environment}`)
                            is not supported. This buildpack currently supports `production` and
                            `test` execution environments.
                        "},
                        None,
                    );
                }
            },
        },
        DotnetBuildpackError::RestoreDotnetToolsCommand(error) => match error {
            fun_run::CmdError::SystemError(_message, io_error) => log_io_error_to(
                &mut writer,
                "Unable to restore .NET tools",
                "running the command to restore .NET tools",
                io_error,
            ),
            fun_run::CmdError::NonZeroExitNotStreamed(output)
            | fun_run::CmdError::NonZeroExitAlreadyStreamed(output) => log_error_to(
                &mut writer,
                "Unable to restore .NET tools",
                formatdoc! {"
                    The `dotnet tool restore` command failed ({exit_status}).

                    The most common cause is a configuration error in your tool manifest. Review
                    the command output above to find and fix the issue.

                    The failure may also be temporary due to a network or service outage. Retrying
                    your build often resolves this.

                    If the log suggests a NuGet issue, check the service status before retrying:
                    https://status.nuget.org
                ", exit_status = output.status()},
                None,
            ),
        },
        DotnetBuildpackError::PublishCommand(error) => match error {
            fun_run::CmdError::SystemError(_message, io_error) => log_io_error_to(
                &mut writer,
                "Unable to publish",
                "running the command to publish the .NET solution/project",
                io_error,
            ),
            fun_run::CmdError::NonZeroExitNotStreamed(output)
            | fun_run::CmdError::NonZeroExitAlreadyStreamed(output) => log_error_to(
                &mut writer,
                "Unable to publish",
                formatdoc! {"
                    The `dotnet publish` command failed ({exit_status}).

                    The most common cause is a compilation error. Review the command output above
                    to find and fix the issue.

                    The failure may also be temporary due to a network or service outage. Retrying
                    your build often resolves this.

                    If the log suggests a NuGet issue, check the service status before retrying:
                    https://status.nuget.org
                ", exit_status = output.status()},
                None,
            ),
        },
        DotnetBuildpackError::CopyRuntimeFiles(io_error) => log_io_error_to(
            &mut writer,
            "Error copying .NET runtime files",
            "copying .NET runtime files from the SDK layer to the runtime layer",
            io_error,
        ),
    }
}

fn on_load_dotnet_project_error_with_writer(
    mut writer: impl Write,
    error: &project::LoadError,
    occurred_while: &str,
) {
    match error {
        project::LoadError::ReadProjectFile(io_error) => {
            log_io_error_to(
                &mut writer,
                "Error loading the project file",
                occurred_while,
                io_error,
            );
        }
        project::LoadError::XmlParseError(xml_parse_error) => log_error_to(
            &mut writer,
            "Error parsing the project file",
            formatdoc! {"
                We can’t parse the project file’s XML content. Parsing errors usually
                indicate an error in the project file.
                
                Use the debug information above to troubleshoot and retry your build."},
            Some(xml_parse_error.to_string()),
        ),
        project::LoadError::MissingTargetFramework(project_path) => {
            log_error_to(
                &mut writer,
                "Project file missing TargetFramework property",
                formatdoc! {"
                    The project file `{project_path}` is missing the `TargetFramework` property.
                    You must set this required property.

                    For more information, see:
                    https://github.com/heroku/buildpacks-dotnet#net-version
                ", project_path = project_path.to_string_lossy()},
                None,
            );
        }
    }
}

fn log_io_error_to(
    mut writer: impl Write,
    header: &str,
    occurred_while: &str,
    io_error: &io::Error,
) {
    log_error_to(
        &mut writer,
        header,
        formatdoc! {"
            An unexpected I/O error occurred while {occurred_while}.

            Use the debug information above to troubleshoot and retry your build. If the
            issue persists, file an issue here:
            https://github.com/heroku/buildpacks-dotnet/issues/new
        "},
        Some(io_error.to_string()),
    );
}

fn log_error_to(
    mut writer: impl Write,
    header: impl AsRef<str>,
    body: impl AsRef<str>,
    error: Option<String>,
) {
    let mut log = Print::new(vec![]).without_header();
    if let Some(error) = error {
        let bullet = log.bullet(style::important("Debug info"));
        log = bullet.sub_bullet(error).done();
    }
    let _ = writer.write_all(&log.error(formatdoc! {"
        {header}

        {body}
    ", header = header.as_ref(), body = body.as_ref(),
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::{assert_snapshot, with_settings};
    use std::path::PathBuf;

    #[test]
    fn test_libcnb_internal_buildpack_error() {
        assert_writer_snapshot(|writer| {
            on_error_with_writer(
                libcnb::Error::<DotnetBuildpackError>::CannotCreatePlatformFromPath(
                    create_io_error(),
                ),
                writer,
            );
        });
    }

    #[test]
    fn test_buildpack_detection_error() {
        assert_error_snapshot(DotnetBuildpackError::BuildpackDetection(create_io_error()));
    }

    #[test]
    fn test_read_project_toml_file_error() {
        assert_error_snapshot(DotnetBuildpackError::ReadProjectTomlFile(create_io_error()));
    }

    #[test]
    fn test_parse_project_toml_error() {
        assert_error_snapshot(DotnetBuildpackError::ParseProjectToml(
            toml::from_str::<toml::Value>("foo").unwrap_err(),
        ));
    }

    #[test]
    fn test_no_solution_projects_error() {
        assert_error_snapshot(DotnetBuildpackError::NoSolutionProjects(PathBuf::from(
            "/foo/bar.sln",
        )));
    }

    #[test]
    fn test_multiple_root_directory_solution_files_error() {
        assert_error_snapshot(DotnetBuildpackError::MultipleRootDirectorySolutionFiles(
            vec![PathBuf::from("foo.sln"), PathBuf::from("bar.sln")],
        ));
    }

    #[test]
    fn test_multiple_root_directory_project_files_error() {
        assert_error_snapshot(DotnetBuildpackError::MultipleRootDirectoryProjectFiles(
            vec![PathBuf::from("foo.csproj"), PathBuf::from("bar.fsproj")],
        ));
    }

    #[test]
    fn test_load_solution_file_read_error() {
        assert_error_snapshot(DotnetBuildpackError::LoadSolutionFile(
            solution::LoadError::ReadSolutionFile(create_io_error()),
        ));
    }

    #[test]
    fn test_load_solution_file_project_not_found_error() {
        assert_error_snapshot(DotnetBuildpackError::LoadSolutionFile(
            solution::LoadError::ProjectNotFound(std::path::PathBuf::from(
                "src/MyProject/MyProject.csproj",
            )),
        ));
    }

    #[test]
    fn test_load_solution_file_slnx_parse_error() {
        assert_error_snapshot(DotnetBuildpackError::LoadSolutionFile(
            solution::LoadError::SlnxParseError(quick_xml::DeError::Custom(
                "XML parsing error".to_string(),
            )),
        ));
    }

    #[test]
    fn test_load_solution_file_load_project_read_error() {
        assert_error_snapshot(DotnetBuildpackError::LoadSolutionFile(
            solution::LoadError::LoadProject(
                project::LoadError::ReadProjectFile(create_io_error()),
            ),
        ));
    }

    #[test]
    fn test_load_solution_file_load_project_xml_parse_error() {
        assert_error_snapshot(DotnetBuildpackError::LoadSolutionFile(
            solution::LoadError::LoadProject(project::LoadError::XmlParseError(
                create_xml_parse_error(),
            )),
        ));
    }

    #[test]
    fn test_load_project_file_read_error() {
        assert_error_snapshot(DotnetBuildpackError::LoadProjectFile(
            project::LoadError::ReadProjectFile(create_io_error()),
        ));
    }

    #[test]
    fn test_load_solution_file_load_project_missing_target_framework_error() {
        assert_error_snapshot(DotnetBuildpackError::LoadSolutionFile(
            solution::LoadError::LoadProject(project::LoadError::MissingTargetFramework(
                PathBuf::from("foo.csproj"),
            )),
        ));
    }

    #[test]
    fn test_load_project_file_xml_parse_error() {
        assert_error_snapshot(DotnetBuildpackError::LoadProjectFile(
            project::LoadError::XmlParseError(create_xml_parse_error()),
        ));
    }

    #[test]
    fn test_load_project_file_missing_target_framework_error() {
        assert_error_snapshot(DotnetBuildpackError::LoadProjectFile(
            project::LoadError::MissingTargetFramework(PathBuf::from("fpp.csproj")),
        ));
    }

    #[test]
    fn test_parse_target_framework_moniker_invalid_format_error() {
        assert_error_snapshot(DotnetBuildpackError::ParseTargetFrameworkMoniker(
            ParseTargetFrameworkError::InvalidFormat("netfoo".to_string()),
        ));
    }

    #[test]
    fn test_parse_target_framework_moniker_unsupported_os_tfm_error() {
        assert_error_snapshot(DotnetBuildpackError::ParseTargetFrameworkMoniker(
            ParseTargetFrameworkError::UnsupportedOSTfm("net8.0-windows".to_string()),
        ));
    }

    #[test]
    fn test_read_global_json_file_error() {
        assert_error_snapshot(DotnetBuildpackError::ReadGlobalJsonFile(create_io_error()));
    }

    #[test]
    fn test_parse_global_json_error() {
        assert_error_snapshot(DotnetBuildpackError::ParseGlobalJson(
            serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
        ));
    }

    #[test]
    fn test_parse_global_json_version_requirement_error() {
        assert_error_snapshot(DotnetBuildpackError::ParseGlobalJsonVersionRequirement(
            semver::VersionReq::parse("invalid-version").unwrap_err(),
        ));
    }

    #[test]
    fn test_parse_inventory_error() {
        assert_error_snapshot(DotnetBuildpackError::ParseInventory(
            libherokubuildpack::inventory::ParseInventoryError::TomlError(
                toml::from_str::<toml::Value>("invalid toml").unwrap_err(),
            ),
        ));
    }

    #[test]
    fn test_parse_solution_version_requirement_error() {
        assert_error_snapshot(DotnetBuildpackError::ParseSolutionVersionRequirement(
            semver::VersionReq::parse("invalid-version").unwrap_err(),
        ));
    }

    #[test]
    fn test_resolve_sdk_version_error() {
        assert_error_snapshot(DotnetBuildpackError::ResolveSdkVersion(
            semver::VersionReq::parse("~4.8").unwrap(),
        ));
    }

    #[test]
    fn test_sdk_layer_download_archive_http_error() {
        assert_error_snapshot(DotnetBuildpackError::SdkLayer(
            SdkLayerError::DownloadArchive(libherokubuildpack::download::DownloadError::HttpError(
                reqwest::blocking::get("").unwrap_err(), // An empty URL will return a "builder error" (without making any requests).
            )),
        ));
    }

    #[test]
    fn test_sdk_layer_download_archive_io_error() {
        assert_error_snapshot(DotnetBuildpackError::SdkLayer(
            SdkLayerError::DownloadArchive(libherokubuildpack::download::DownloadError::IoError(
                create_io_error(),
            )),
        ));
    }

    #[test]
    fn test_sdk_layer_decompress_archive_error() {
        assert_error_snapshot(DotnetBuildpackError::SdkLayer(
            SdkLayerError::DecompressArchive(create_io_error()),
        ));
    }

    #[test]
    fn test_sdk_layer_verify_archive_checksum_error() {
        assert_error_snapshot(DotnetBuildpackError::SdkLayer(
            SdkLayerError::VerifyArchiveChecksum {
                expected: vec![0xAA, 0xBB, 0xCC, 0xDD],
                actual: vec![0x11, 0x22, 0x33, 0x44],
            },
        ));
    }

    #[test]
    fn test_sdk_layer_open_archive_error() {
        assert_error_snapshot(DotnetBuildpackError::SdkLayer(SdkLayerError::OpenArchive(
            create_io_error(),
        )));
    }

    #[test]
    fn test_sdk_layer_read_archive_error() {
        assert_error_snapshot(DotnetBuildpackError::SdkLayer(SdkLayerError::ReadArchive(
            create_io_error(),
        )));
    }

    #[test]
    fn test_parse_buildpack_configuration_invalid_msbuild_verbosity_level_error() {
        assert_error_snapshot(DotnetBuildpackError::ParseBuildpackConfiguration(
            DotnetBuildpackConfigurationError::VerbosityLevel(ParseVerbosityLevelError(
                "Foo".to_string(),
            )),
        ));
    }

    #[test]
    fn test_parse_buildpack_configuration_unsupported_execution_environment_error() {
        assert_error_snapshot(DotnetBuildpackError::ParseBuildpackConfiguration(
            DotnetBuildpackConfigurationError::ExecutionEnvironment(
                ExecutionEnvironmentError::UnsupportedExecutionEnvironment("foo".to_string()),
            ),
        ));
    }

    #[test]
    fn test_restore_dotnet_tools_command_system_error() {
        assert_error_snapshot(DotnetBuildpackError::RestoreDotnetToolsCommand(
            fun_run::CmdError::SystemError(
                "Failed to start process".to_string(),
                create_io_error(),
            ),
        ));
    }

    #[test]
    fn test_restore_dotnet_tools_command_non_zero_exit_not_streamed_error() {
        assert_error_snapshot(DotnetBuildpackError::RestoreDotnetToolsCommand(
            create_cmd_error(1),
        ));
    }

    #[test]
    fn test_publish_command_system_error() {
        assert_error_snapshot(DotnetBuildpackError::PublishCommand(
            fun_run::CmdError::SystemError("Cannot find executable".to_string(), create_io_error()),
        ));
    }

    #[test]
    fn test_publish_command_non_zero_exit_not_streamed_error() {
        assert_error_snapshot(DotnetBuildpackError::PublishCommand(create_cmd_error(5)));
    }

    #[test]
    fn test_copy_runtime_files_error() {
        assert_error_snapshot(DotnetBuildpackError::CopyRuntimeFiles(create_io_error()));
    }

    fn assert_error_snapshot(error: DotnetBuildpackError) {
        assert_writer_snapshot(|writer| {
            on_error_with_writer(libcnb::Error::BuildpackError(error), writer);
        });
    }

    fn assert_writer_snapshot(function: impl FnOnce(&mut dyn Write)) {
        let output = {
            let mut buffer = Vec::new();
            function(&mut buffer);
            String::from_utf8(buffer).unwrap()
        };

        with_settings!({
            prepend_module_to_snapshot => false,
            omit_expression => true,
        }, {
            assert_snapshot!(snapshot_name(), output);
        });
    }

    fn snapshot_name() -> String {
        std::thread::current()
            .name()
            .expect("Test name should be available as the current thread name")
            .rsplit("::")
            .next()
            .unwrap()
            .trim_start_matches("test_")
            .to_string()
    }

    fn create_io_error() -> io::Error {
        std::io::Error::other("foo bar baz")
    }

    fn create_xml_parse_error() -> quick_xml::de::DeError {
        quick_xml::de::DeError::Custom("Simulated XML parsing error".to_string())
    }

    fn create_cmd_error(exit_code: i32) -> fun_run::CmdError {
        fun_run::nonzero_captured(
            "foo".to_string(),
            std::process::Output {
                status: std::os::unix::process::ExitStatusExt::from_raw(exit_code),
                stdout: vec![],
                stderr: vec![],
            },
        )
        .unwrap_err()
    }
}
