use libcnb::data::launch::{Process, ProcessBuilder};
use libcnb::data::process_type;

use crate::dotnet::runtime_identifier::RuntimeIdentifier;
use crate::dotnet_buildpack_configuration::VerbosityLevel;
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
        };
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
        let mut args = vec![format!(
            "dotnet test {}",
            value
                .path
                .file_name()
                .expect("Solution to have a file name")
                .to_string_lossy()
        )];
        if let Some(configuration) = value.configuration {
            args.push(format!("--configuration {configuration}"));
        }
        if let Some(verbosity_level) = value.verbosity_level {
            args.push(format!("--verbosity {verbosity_level}"));
        }
        ProcessBuilder::new(process_type!("test"), ["bash", "-c", &args.join(" ")]).build()
    }
}
