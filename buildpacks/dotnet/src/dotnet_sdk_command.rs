use crate::dotnet::runtime_identifier::RuntimeIdentifier;
use crate::dotnet_buildpack_configuration::VerbosityLevel;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub(crate) enum DotnetSdkCommand {
    Publish {
        path: PathBuf,
        runtime_identifier: RuntimeIdentifier,
        configuration: Option<String>,
        verbosity_level: Option<VerbosityLevel>,
    },
}

impl DotnetSdkCommand {
    pub(crate) fn name(&self) -> &str {
        match self {
            DotnetSdkCommand::Publish { .. } => "Publish",
        }
    }
}

impl From<DotnetSdkCommand> for Command {
    fn from(value: DotnetSdkCommand) -> Self {
        let mut command = Command::new("dotnet");
        match &value {
            DotnetSdkCommand::Publish {
                path,
                runtime_identifier,
                configuration,
                verbosity_level,
            } => {
                command.args([
                    value.name().to_lowercase().as_str(),
                    &path.to_string_lossy(),
                    "--runtime",
                    &runtime_identifier.to_string(),
                    "-p:PublishDir=bin/publish",
                ]);

                if let Some(configuration) = configuration {
                    command.args(["--configuration", configuration]);
                }
                if let Some(verbosity_level) = verbosity_level {
                    command.args(["--verbosity", &verbosity_level.to_string()]);
                };
                command
            }
        }
    }
}
