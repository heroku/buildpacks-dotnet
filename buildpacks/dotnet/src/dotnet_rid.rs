use std::env;
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
    Unknown,
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
            RuntimeIdentifier::Unknown => write!(f, "unknown"),
        }
    }
}

/// This function returns the .NET Runtime Identifier (RID)
/// based on the current operating system and architecture.
///
/// It supports the following RIDs:
/// - `linux-x64`
/// - `linux-arm64`
/// - `linux-musl-x64`
/// - `linux-musl-arm64`
/// - `osx-x64`
/// - `osx-arm64`
///
/// Other combinations of OS and architecture will return `Unknown`.
pub(crate) fn get_dotnet_rid() -> RuntimeIdentifier {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    match (os, arch) {
        ("linux", "x86_64") => {
            if is_musl() {
                RuntimeIdentifier::LinuxMuslX64
            } else {
                RuntimeIdentifier::LinuxX64
            }
        }
        ("linux", "aarch64") => {
            if is_musl() {
                RuntimeIdentifier::LinuxMuslArm64
            } else {
                RuntimeIdentifier::LinuxArm64
            }
        }
        ("macos", "x86_64") => RuntimeIdentifier::OsxX64,
        ("macos", "aarch64") => RuntimeIdentifier::OsxArm64,
        _ => RuntimeIdentifier::Unknown,
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
    use std::env::consts::{ARCH, OS};

    use super::*;

    #[test]
    fn test_get_dotnet_rid_linux_x64() {
        if OS == "linux" && ARCH == "x86_64" && !is_musl() {
            assert_eq!(get_dotnet_rid(), RuntimeIdentifier::LinuxX64);
        }
    }

    #[test]
    fn test_get_dotnet_rid_linux_arm64() {
        if OS == "linux" && ARCH == "aarch64" && !is_musl() {
            assert_eq!(get_dotnet_rid(), RuntimeIdentifier::LinuxArm64);
        }
    }

    #[test]
    fn test_get_dotnet_rid_linux_musl_x64() {
        if OS == "linux" && ARCH == "x86_64" && is_musl() {
            assert_eq!(get_dotnet_rid(), RuntimeIdentifier::LinuxMuslX64);
        }
    }

    #[test]
    fn test_get_dotnet_rid_linux_musl_arm64() {
        if OS == "linux" && ARCH == "aarch64" && is_musl() {
            assert_eq!(get_dotnet_rid(), RuntimeIdentifier::LinuxMuslArm64);
        }
    }

    #[test]
    fn test_get_dotnet_rid_osx_x64() {
        if OS == "macos" && ARCH == "x86_64" {
            assert_eq!(get_dotnet_rid(), RuntimeIdentifier::OsxX64);
        }
    }

    #[test]
    fn test_get_dotnet_rid_osx_arm64() {
        if OS == "macos" && ARCH == "aarch64" {
            assert_eq!(get_dotnet_rid(), RuntimeIdentifier::OsxArm64);
        }
    }

    #[test]
    fn test_get_dotnet_rid_unknown() {
        if OS != "linux" && OS != "macos" {
            assert_eq!(get_dotnet_rid(), RuntimeIdentifier::Unknown);
        }
    }
}
