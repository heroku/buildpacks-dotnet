use crate::dotnet::runtime_identifier::RuntimeIdentifier;
use std::fmt;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum VerbosityLevel {
    Quiet,
    Minimal,
    Normal,
    Detailed,
    Diagnostic,
}

impl fmt::Display for VerbosityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerbosityLevel::Quiet => write!(f, "quiet"),
            VerbosityLevel::Minimal => write!(f, "minimal"),
            VerbosityLevel::Normal => write!(f, "normal"),
            VerbosityLevel::Detailed => write!(f, "detailed"),
            VerbosityLevel::Diagnostic => write!(f, "diagnostic"),
        }
    }
}
