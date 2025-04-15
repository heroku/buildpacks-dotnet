use inventory::artifact::{Arch, Os};
use libherokubuildpack::inventory;
use std::fmt;

/// Enum representing supported .NET Runtime Identifiers (RIDs).
#[derive(Debug, PartialEq)]
pub(crate) enum RuntimeIdentifier {
    LinuxX64,
    LinuxArm64,
    OsxX64,
    OsxArm64,
}

impl fmt::Display for RuntimeIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeIdentifier::LinuxX64 => write!(f, "linux-x64"),
            RuntimeIdentifier::LinuxArm64 => write!(f, "linux-arm64"),
            RuntimeIdentifier::OsxX64 => write!(f, "osx-x64"),
            RuntimeIdentifier::OsxArm64 => write!(f, "osx-arm64"),
        }
    }
}

pub(crate) fn get_runtime_identifier(os: Os, arch: Arch) -> RuntimeIdentifier {
    match (os, arch) {
        (Os::Linux, Arch::Amd64) => RuntimeIdentifier::LinuxX64,
        (Os::Linux, Arch::Arm64) => RuntimeIdentifier::LinuxArm64,
        (Os::Darwin, Arch::Amd64) => RuntimeIdentifier::OsxX64,
        (Os::Darwin, Arch::Arm64) => RuntimeIdentifier::OsxArm64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_identifier_display() {
        assert_eq!(RuntimeIdentifier::LinuxX64.to_string(), "linux-x64");
        assert_eq!(RuntimeIdentifier::LinuxArm64.to_string(), "linux-arm64");
        assert_eq!(RuntimeIdentifier::OsxX64.to_string(), "osx-x64");
        assert_eq!(RuntimeIdentifier::OsxArm64.to_string(), "osx-arm64");
    }

    #[test]
    fn test_get_runtime_identifier() {
        assert_eq!(
            get_runtime_identifier(Os::Linux, Arch::Amd64),
            RuntimeIdentifier::LinuxX64
        );
        assert_eq!(
            get_runtime_identifier(Os::Linux, Arch::Arm64),
            RuntimeIdentifier::LinuxArm64
        );
        assert_eq!(
            get_runtime_identifier(Os::Darwin, Arch::Amd64),
            RuntimeIdentifier::OsxX64
        );
        assert_eq!(
            get_runtime_identifier(Os::Darwin, Arch::Arm64),
            RuntimeIdentifier::OsxArm64
        );
    }
}
