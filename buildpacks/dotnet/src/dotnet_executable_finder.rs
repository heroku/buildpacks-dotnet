use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::dotnet_project::{DotnetProject, ProjectType};
use crate::dotnet_rid::RuntimeIdentifier;

#[derive(Error, Debug)]
pub(crate) enum ExecutablePathError {
    #[error("Project type does not produce an executable")]
    InvalidProjectType,
    #[error("Failed to determine executable path")]
    PathError,
}

/// Determines the path to the framework-dependent executable for the .NET project.
///
/// # Arguments
///
/// * `project` - A reference to a `DotnetProject` instance.
/// * `project_file_path` - The path to the .NET project file.
/// * `configuration` - The build configuration (e.g., "Release" or "Debug").
/// * `rid` - The Runtime Identifier (e.g., "linux-x64").
///
/// # Returns
///
/// * A `Result` containing the path to the executable file or an error message.
pub(crate) fn determine_executable_path(
    project: &DotnetProject,
    project_file_path: &Path,
    configuration: &str,
    rid: &RuntimeIdentifier,
) -> Result<PathBuf, ExecutablePathError> {
    // Construct the output directory based on project properties
    let output_dir = project_file_path
        .parent()
        .ok_or(ExecutablePathError::PathError)?
        .join("bin")
        .join(configuration)
        .join(&project.target_framework)
        .join(rid.to_string())
        .join("publish");

    // Determine the executable name
    let executable_name = match &project.assembly_name {
        Some(name) if !name.is_empty() => name.clone(),
        _ => project_file_path
            .file_stem()
            .expect("project file path to have a file name")
            .to_string_lossy()
            .to_string(),
    };

    // Construct the full path to the executable
    let executable_path = match project.project_type {
        ProjectType::ConsoleApplication
        | ProjectType::WebApplication
        | ProjectType::RazorApplication
        | ProjectType::BlazorWebAssembly
        | ProjectType::Worker => output_dir.join(executable_name),
        ProjectType::Library | ProjectType::Unknown => {
            return Err(ExecutablePathError::InvalidProjectType)
        }
    };

    Ok(executable_path)
}

#[cfg(test)]
mod tests {
    use crate::dotnet_project::ProjectType;

    use super::*;

    #[test]
    fn test_determine_executable_path_with_assembly_name() {
        let project = DotnetProject {
            sdk_id: "Microsoft.NET.Sdk.Web".to_string(),
            target_framework: "net6.0".to_string(),
            project_type: ProjectType::WebApplication,
            assembly_name: Some("WebApiExecutable".to_string()),
        };
        let project_file_path = Path::new("WebApi/WebApi.csproj");

        let result = determine_executable_path(
            &project,
            project_file_path,
            "Release",
            &RuntimeIdentifier::LinuxX64,
        )
        .unwrap();
        let expected_path =
            Path::new("WebApi/bin/Release/net6.0/linux-x64/publish/WebApiExecutable");

        assert_eq!(result, expected_path);
    }

    #[test]
    fn test_determine_executable_path_without_assembly_name() {
        let project = DotnetProject {
            sdk_id: "Microsoft.NET.Sdk.Web".to_string(),
            target_framework: "net6.0".to_string(),
            project_type: ProjectType::WebApplication,
            assembly_name: None,
        };
        let project_file_path = Path::new("WebApi/WebApi.csproj");

        let result = determine_executable_path(
            &project,
            project_file_path,
            "Release",
            &RuntimeIdentifier::LinuxArm64,
        )
        .unwrap();
        let expected_path = Path::new("WebApi/bin/Release/net6.0/linux-arm64/publish/WebApi");

        assert_eq!(result, expected_path);
    }

    #[test]
    fn test_invalid_project_type() {
        let project = DotnetProject {
            sdk_id: "Microsoft.NET.Sdk".to_string(),
            target_framework: "net6.0".to_string(),
            project_type: ProjectType::Library,
            assembly_name: Some("LibraryProject".to_string()),
        };
        let project_file_path = Path::new("Library/Library.csproj");

        let result = determine_executable_path(
            &project,
            project_file_path,
            "Release",
            &RuntimeIdentifier::LinuxX64,
        );

        assert!(matches!(
            result,
            Err(ExecutablePathError::InvalidProjectType)
        ));
    }

    #[test]
    fn test_path_error() {
        let project = DotnetProject {
            sdk_id: "Microsoft.NET.Sdk.Web".to_string(),
            target_framework: "net6.0".to_string(),
            project_type: ProjectType::WebApplication,
            assembly_name: Some("WebApiExecutable".to_string()),
        };
        let project_file_path = Path::new(""); // Invalid path to induce an error

        let result = determine_executable_path(
            &project,
            project_file_path,
            "Release",
            &RuntimeIdentifier::LinuxX64,
        );

        assert!(matches!(result, Err(ExecutablePathError::PathError)));
    }
}
