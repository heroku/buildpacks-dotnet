use crate::DotnetBuildpackError;
use crate::dotnet::target_framework_moniker::ParseTargetFrameworkError;
use crate::dotnet::{project, solution};
use crate::dotnet_buildpack_configuration::{
    DotnetBuildpackConfigurationError, ExecutionEnvironmentError,
};
use crate::layers::sdk::SdkLayerError;
use bullet_stream::{Print, style};
use indoc::formatdoc;
use std::io::{self, Write, stderr};

pub(crate) fn on_error(error: libcnb::Error<DotnetBuildpackError>) {
    on_error_with_writer(error, stderr());
}

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
            "determining if we must run the Heroku .NET buildpack for this application.",
            io_error,
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
                        buildpack currently supports the following TFMs: `net5.0`, `net6.0`, `net7.0`, `net8.0`.

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
            DotnetBuildpackConfigurationError::InvalidMsbuildVerbosityLevel(verbosity_level) => {
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
            DotnetBuildpackConfigurationError::ExecutionEnvironmentError(error) => match error {
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
                    The `dotnet tool restore` command exited unsuccessfully ({exit_status}).

                    This error usually happens due to configuration errors. Use the command output
                    above to troubleshoot and retry your build.

                    Restoring .NET tools can also fail for a number of other reasons, such as
                    intermittent network issues, unavailability of the NuGet package feed and/or
                    other external dependencies, etc.

                    Try again to see if the error resolves itself.
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
                    The `dotnet publish` command exited unsuccessfully ({exit_status}).

                    This error usually happens due to compilation errors. Use the command output
                    above to troubleshoot and retry your build.

                    The publish process can also fail for a number of other reasons, such as
                    intermittent network issues, unavailability of the NuGet package feed and/or
                    other external dependencies, etc.

                    Try again to see if the error resolves itself.
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
    use roxmltree::TextPos;
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
        assert_error_snapshot(&DotnetBuildpackError::BuildpackDetection(create_io_error()));
    }

    #[test]
    fn test_no_solution_projects_error() {
        assert_error_snapshot(&DotnetBuildpackError::NoSolutionProjects(PathBuf::from(
            "/foo/bar.sln",
        )));
    }

    #[test]
    fn test_multiple_root_directory_project_files_error() {
        assert_error_snapshot(&DotnetBuildpackError::MultipleRootDirectoryProjectFiles(
            vec![PathBuf::from("foo.csproj"), PathBuf::from("bar.fsproj")],
        ));
    }

    #[test]
    fn test_load_solution_file_read_error() {
        assert_error_snapshot(&DotnetBuildpackError::LoadSolutionFile(
            solution::LoadError::ReadSolutionFile(create_io_error()),
        ));
    }

    #[test]
    fn test_load_solution_file_load_project_read_error() {
        assert_error_snapshot(&DotnetBuildpackError::LoadSolutionFile(
            solution::LoadError::LoadProject(
                project::LoadError::ReadProjectFile(create_io_error()),
            ),
        ));
    }

    #[test]
    fn test_load_solution_file_load_project_xml_parse_error() {
        assert_error_snapshot(&DotnetBuildpackError::LoadSolutionFile(
            solution::LoadError::LoadProject(project::LoadError::XmlParseError(
                create_xml_parse_error(),
            )),
        ));
    }

    #[test]
    fn test_load_project_file_read_error() {
        assert_error_snapshot(&DotnetBuildpackError::LoadProjectFile(
            project::LoadError::ReadProjectFile(create_io_error()),
        ));
    }

    #[test]
    fn test_load_solution_file_load_project_missing_target_framework_error() {
        assert_error_snapshot(&DotnetBuildpackError::LoadSolutionFile(
            solution::LoadError::LoadProject(project::LoadError::MissingTargetFramework(
                PathBuf::from("foo.csproj"),
            )),
        ));
    }

    #[test]
    fn test_parse_global_json_error() {
        assert_error_snapshot(&DotnetBuildpackError::ParseGlobalJson(
            serde::de::Error::custom("foo"),
        ));
    }

    fn assert_error_snapshot(error: &DotnetBuildpackError) {
        assert_writer_snapshot(|writer| on_buildpack_error_with_writer(error, writer));
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

    fn create_io_error() -> io::Error {
        std::io::Error::new(io::ErrorKind::Other, "foo bar baz")
    }

    fn create_xml_parse_error() -> roxmltree::Error {
        roxmltree::Error::InvalidString("Simulated XML parsing error at line 1", TextPos::new(1, 2))
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
}
