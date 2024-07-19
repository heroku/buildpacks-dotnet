use crate::dotnet::runtime_identifier::RuntimeIdentifier;
use std::fmt;
use std::path::PathBuf;
use std::process::Command;

pub(crate) struct DotnetPublishCommand {
    pub(crate) path: PathBuf,
    pub(crate) configuration: String,
    pub(crate) runtime_identifier: RuntimeIdentifier,
    pub(crate) verbosity_level: VerbosityLevel,
}

impl From<DotnetPublishCommand> for Command {
    fn from(value: DotnetPublishCommand) -> Self {
        let mut command = Command::new("dotnet");
        command.args([
            "publish",
            &value.path.to_string_lossy(),
            "--configuration",
            &value.configuration,
            "--runtime",
            &value.runtime_identifier.to_string(),
            "--verbosity",
            &value.verbosity_level.to_string(),
        ]);
        command
    }
}

#[derive(Clone, Copy)]
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
