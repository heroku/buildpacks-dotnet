use crate::dotnet::runtime_identifier::RuntimeIdentifier;
use crate::dotnet_buildpack_configuration::VerbosityLevel;
use libcnb::data::launch::{Process, ProcessBuilder};
use libcnb::data::process_type;
use std::env::temp_dir;
use std::path::PathBuf;
use std::process::Command;

pub(crate) struct DotnetPublishCommand {
    pub(crate) path: PathBuf,
    pub(crate) runtime_identifier: RuntimeIdentifier,
    pub(crate) configuration: Option<String>,
    pub(crate) verbosity_level: Option<VerbosityLevel>,
}

impl From<DotnetPublishCommand> for Command {
    fn from(value: DotnetPublishCommand) -> Self {
        let mut command = Command::new("dotnet");
        command.args([
            "publish",
            &value.path.to_string_lossy(),
            "--runtime",
            &value.runtime_identifier.to_string(),
            "-p:PublishDir=bin/publish",
            "--artifacts-path",
            &temp_dir().join("build_artifacts").to_string_lossy(),
        ]);

        if let Some(configuration) = value.configuration {
            command.args(["--configuration", &configuration]);
        }
        if let Some(verbosity_level) = value.verbosity_level {
            command.args(["--verbosity", &verbosity_level.to_string()]);
        }
        command
    }
}

pub(crate) struct DotnetTestCommand {
    pub(crate) path: PathBuf,
    pub(crate) configuration: Option<String>,
    pub(crate) verbosity_level: Option<VerbosityLevel>,
}

impl From<DotnetTestCommand> for Process {
    fn from(value: DotnetTestCommand) -> Self {
        let mut command = vec![
            "dotnet".to_string(),
            "test".to_string(),
            value
                .path
                .file_name()
                .expect("Solution to have a file name")
                .to_string_lossy()
                .to_string(),
        ];
        if let Some(configuration) = value.configuration {
            command.extend(["--configuration".to_string(), configuration]);
        }
        if let Some(verbosity_level) = value.verbosity_level {
            command.extend(["--verbosity".to_string(), verbosity_level.to_string()]);
        }
        ProcessBuilder::new(process_type!("test"), command).build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libcnb::data::launch::{Process, WorkingDirectory};
    use libcnb::data::process_type;
    use std::path::PathBuf;

    #[test]
    fn test_process_from_dotnet_test_command() {
        let test_command = base_test_command();
        let process = Process::from(test_command);
        assert_test_process(&process, &base_test_command_args());
    }

    #[test]
    fn test_process_from_dotnet_test_command_with_spaces_in_path() {
        let mut test_command = base_test_command();
        test_command.path = PathBuf::from("/foo/bar baz.sln");

        let process = Process::from(test_command);
        assert_test_process(
            &process,
            &[
                "dotnet".to_string(),
                "test".to_string(),
                "bar baz.sln".to_string(),
            ],
        );
    }

    #[test]
    fn test_process_from_dotnet_test_command_with_configuration_and_verbosity_level() {
        let mut test_command = base_test_command();
        test_command.configuration = Some("Release".to_string());
        test_command.verbosity_level = Some(VerbosityLevel::Normal);

        let process = Process::from(test_command);
        let mut expected_args = base_test_command_args();
        expected_args.extend(vec![
            "--configuration".to_string(),
            "Release".to_string(),
            "--verbosity".to_string(),
            "normal".to_string(),
        ]);
        assert_test_process(&process, &expected_args);
    }

    fn assert_test_process(process: &Process, expected_command: &[String]) {
        assert_eq!(process.r#type, process_type!("test"));
        assert_eq!(process.command, expected_command);
        assert_eq!(process.args, Vec::<String>::new());
        assert!(!process.default);
        assert_eq!(process.working_directory, WorkingDirectory::App);
    }

    fn base_test_command() -> DotnetTestCommand {
        DotnetTestCommand {
            path: PathBuf::from("/foo/bar.sln"),
            configuration: None,
            verbosity_level: None,
        }
    }

    fn base_test_command_args() -> Vec<String> {
        vec![
            "dotnet".to_string(),
            "test".to_string(),
            "bar.sln".to_string(),
        ]
    }

    #[test]
    fn test_command_from_dotnet_publish_command() {
        let publish_command = base_publish_command();
        let command = Command::from(publish_command);
        assert_publish_command_args(&command, &base_publish_command_args());
    }

    #[test]
    fn test_command_from_dotnet_publish_command_with_configuration_and_verbosity_level() {
        let mut publish_command = base_publish_command();
        publish_command.configuration = Some("Release".to_string());
        publish_command.verbosity_level = Some(VerbosityLevel::Normal);

        let command = Command::from(publish_command);
        let mut expected_args = base_publish_command_args();
        expected_args.extend(vec![
            "--configuration".to_string(),
            "Release".to_string(),
            "--verbosity".to_string(),
            "normal".to_string(),
        ]);
        assert_publish_command_args(&command, &expected_args);
    }

    fn assert_publish_command_args(command: &Command, expected_args: &[String]) {
        assert_eq!(command.get_program(), "dotnet");
        let args: Vec<String> = command
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        assert_eq!(args, expected_args);
    }

    fn base_publish_command() -> DotnetPublishCommand {
        DotnetPublishCommand {
            path: PathBuf::from("/foo/bar.sln"),
            runtime_identifier: RuntimeIdentifier::LinuxX64,
            configuration: None,
            verbosity_level: None,
        }
    }

    fn base_publish_command_args() -> Vec<String> {
        vec![
            "publish".to_string(),
            "/foo/bar.sln".to_string(),
            "--runtime".to_string(),
            "linux-x64".to_string(),
            "-p:PublishDir=bin/publish".to_string(),
            "--artifacts-path".to_string(),
            temp_dir()
                .join("build_artifacts")
                .to_string_lossy()
                .to_string(),
        ]
    }
}
