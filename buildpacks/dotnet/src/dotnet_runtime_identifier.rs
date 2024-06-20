use inventory::artifact::{Arch, Os};
use std::fmt;

/// Enum representing various .NET Runtime Identifiers (RIDs).
#[derive(Debug, PartialEq)]
pub(crate) enum RuntimeIdentifier {
    LinuxX64,
    LinuxArm64,
    LinuxMuslX64,
    LinuxMuslArm64,
    OsxX64,
    OsxArm64,
}

impl fmt::Display for RuntimeIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeIdentifier::LinuxX64 => write!(f, "linux-x64"),
            RuntimeIdentifier::LinuxArm64 => write!(f, "linux-arm64"),
            RuntimeIdentifier::LinuxMuslX64 => write!(f, "linux-musl-x64"),
            RuntimeIdentifier::LinuxMuslArm64 => write!(f, "linux-musl-arm64"),
            RuntimeIdentifier::OsxX64 => write!(f, "osx-x64"),
            RuntimeIdentifier::OsxArm64 => write!(f, "osx-arm64"),
        }
    }
}

pub(crate) fn get_runtime_identifier(os: Os, arch: Arch) -> RuntimeIdentifier {
    match (os, arch) {
        (Os::Linux, Arch::Amd64) => {
            if is_musl() {
                RuntimeIdentifier::LinuxMuslX64
            } else {
                RuntimeIdentifier::LinuxX64
            }
        }
        (Os::Linux, Arch::Arm64) => {
            if is_musl() {
                RuntimeIdentifier::LinuxMuslArm64
            } else {
                RuntimeIdentifier::LinuxArm64
            }
        }
        (Os::Darwin, Arch::Amd64) => RuntimeIdentifier::OsxX64,
        (Os::Darwin, Arch::Arm64) => RuntimeIdentifier::OsxArm64,
    }
}

/// Helper function to determine if the current Linux system is using musl libc.
/// It runs the `ldd --version` command and checks the output for "musl".
///
/// # Returns
/// - `true` if musl libc is detected.
/// - `false` if musl libc is not detected or if the command fails.
fn is_musl() -> bool {
    if let Ok(output) = std::process::Command::new("ldd").arg("--version").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        output_str.contains("musl")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_dotnet_rid_linux_x64() {
        assert_eq!(
            get_runtime_identifier(Os::Linux, Arch::Amd64),
            RuntimeIdentifier::LinuxX64
        );
    }

    #[test]
    fn test_get_dotnet_rid_linux_arm64() {
        assert_eq!(
            get_runtime_identifier(Os::Linux, Arch::Arm64),
            RuntimeIdentifier::LinuxArm64
        );
    }

    #[test]
    fn test_get_dotnet_rid_linux_musl_x64() {
        if is_musl() {
            assert_eq!(
                get_runtime_identifier(Os::Linux, Arch::Amd64),
                RuntimeIdentifier::LinuxMuslX64
            );
        }
    }

    #[test]
    fn test_get_dotnet_rid_linux_musl_arm64() {
        if is_musl() {
            assert_eq!(
                get_runtime_identifier(Os::Linux, Arch::Arm64),
                RuntimeIdentifier::LinuxMuslArm64
            );
        }
    }

    #[test]
    fn test_get_dotnet_rid_osx_x64() {
        assert_eq!(
            get_runtime_identifier(Os::Darwin, Arch::Amd64),
            RuntimeIdentifier::OsxX64
        );
    }

    #[test]
    fn test_get_dotnet_rid_osx_arm64() {
        assert_eq!(
            get_runtime_identifier(Os::Darwin, Arch::Arm64),
            RuntimeIdentifier::OsxArm64
        );
    }
}
