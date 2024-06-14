use crate::dotnet_project::{self, DotnetProject, ProjectType};
use crate::{dotnet_rid, dotnet_solution, DotnetFile, DotnetPublishContext};
use libcnb::data::launch::{
    Process, ProcessBuilder, ProcessType, ProcessTypeError, WorkingDirectory,
};
use libherokubuildpack::log::log_info;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub(crate) enum LaunchProcessError {
    #[error(transparent)]
    DotnetProject(#[from] dotnet_project::ParseError),
    #[error("Error parsing target framework: {0}")]
    ParseSolutionFile(io::Error),
    #[error("Invalid CNB process name: {0}")]
    ProcessName(#[from] ProcessTypeError),
}

impl TryFrom<&DotnetPublishContext> for Vec<Process> {
    type Error = LaunchProcessError;

    fn try_from(context: &DotnetPublishContext) -> Result<Self, Self::Error> {
        match &context.dotnet_file {
            DotnetFile::Solution(path) => {
                log_info("Detecting solution executables");

                dotnet_solution::project_file_paths(path)
                    .map_err(LaunchProcessError::ParseSolutionFile)?
                    .into_iter()
                    .map(|project_path| handle_project_file(&project_path, &context.configuration))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|vecs| vecs.into_iter().flatten().collect())
            }
            DotnetFile::Project(project_path) => {
                handle_project_file(project_path, &context.configuration)
            }
        }
    }
}

fn handle_project_file(
    project_path: &Path,
    configuration: &str,
) -> Result<Vec<Process>, LaunchProcessError> {
    let dotnet_project = DotnetProject::try_from(project_path)?;
    if is_executable_project(&dotnet_project.project_type) {
        let executable_path = get_executable_path(configuration, &dotnet_project, project_path);
        let command = build_launch_command(&dotnet_project, &executable_path);

        Ok(vec![ProcessBuilder::new(
            get_executable_name(&dotnet_project, project_path).parse::<ProcessType>()?,
            ["bash", "-c", &command],
        )
        .working_directory(WorkingDirectory::Directory(
            executable_path
                .parent()
                .expect("Executable to have a parent directory")
                .to_path_buf(),
        ))
        .build()])
    } else {
        log_info(format!(
            "Project \"{}\" is not executable (project type is {:?})",
            project_path.to_string_lossy(),
            dotnet_project.project_type
        ));
        Ok(vec![])
    }
}

fn is_executable_project(project_type: &ProjectType) -> bool {
    matches!(
        project_type,
        ProjectType::ConsoleApplication
            | ProjectType::WebApplication
            | ProjectType::RazorApplication
            | ProjectType::Worker
    )
}

fn get_executable_name(dotnet_project: &DotnetProject, project_path: &Path) -> String {
    dotnet_project
        .assembly_name
        .clone()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| {
            project_path
                .file_stem()
                .expect("Project file path to have a file name")
                .to_string_lossy()
                .to_string()
        })
}

fn get_executable_path(
    configuration: &str,
    dotnet_project: &DotnetProject,
    project_path: &Path,
) -> PathBuf {
    project_path
        .parent()
        .expect("Project file will always have a parent directory")
        .join("bin")
        .join(configuration)
        .join(&dotnet_project.target_framework)
        .join(dotnet_rid::get_runtime_identifier().to_string())
        .join("publish")
        .join(get_executable_name(dotnet_project, project_path))
}

fn build_launch_command(dotnet_project: &DotnetProject, executable_path: &Path) -> String {
    let base_command = format!("{}", executable_path.to_string_lossy());

    let executable_name = executable_path
        .file_name()
        .expect("Executable path to have a file name")
        .to_string_lossy();

    match dotnet_project.project_type {
        ProjectType::WebApplication | ProjectType::RazorApplication => {
            log_info(format!(
                "Detected web process type \"{}\" ({})",
                executable_name,
                executable_path.to_string_lossy()
            ));
            format!("{base_command} --urls http://0.0.0.0:$PORT")
        }
        _ => {
            log_info(format!(
                "Detected console process type \"{}\" ({})",
                executable_name,
                executable_path.to_string_lossy()
            ));
            base_command
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_get_executable_name_with_assembly_name() {
        let dotnet_project = DotnetProject {
            assembly_name: Some("TestAssembly".to_string()),
            target_framework: "net6.0".to_string(),
            project_type: ProjectType::ConsoleApplication,
            sdk_id: "Microsoft.NET.Sdk".to_string(),
        };
        let project_path = Path::new("/path/to/project.csproj");
        assert_eq!(
            get_executable_name(&dotnet_project, project_path),
            "TestAssembly"
        );
    }

    #[test]
    fn test_get_executable_name_without_assembly_name() {
        let dotnet_project = DotnetProject {
            assembly_name: None,
            target_framework: "net6.0".to_string(),
            project_type: ProjectType::ConsoleApplication,
            sdk_id: "Microsoft.NET.Sdk".to_string(),
        };
        let project_path = Path::new("/path/to/project.csproj");
        assert_eq!(
            get_executable_name(&dotnet_project, project_path),
            "project"
        );
    }

    #[test]
    fn test_get_executable_path() {
        let dotnet_project = DotnetProject {
            assembly_name: Some("TestAssembly".to_string()),
            target_framework: "net6.0".to_string(),
            project_type: ProjectType::ConsoleApplication,
            sdk_id: "Microsoft.NET.Sdk".to_string(),
        };
        let project_path = Path::new("/path/to/project.csproj");
        let rid = dotnet_rid::get_runtime_identifier().to_string();
        assert_eq!(
            get_executable_path("Release", &dotnet_project, project_path),
            PathBuf::from(format!(
                "/path/to/bin/Release/net6.0/{rid}/publish/TestAssembly"
            ))
        );
    }

    #[test]
    fn test_is_executable_project() {
        assert!(is_executable_project(&ProjectType::ConsoleApplication));
        assert!(is_executable_project(&ProjectType::WebApplication));
        assert!(is_executable_project(&ProjectType::RazorApplication));
        assert!(is_executable_project(&ProjectType::Worker));
        assert!(!is_executable_project(&ProjectType::Library));
        assert!(!is_executable_project(&ProjectType::BlazorWebAssembly));
        assert!(!is_executable_project(&ProjectType::Unknown));
    }

    #[test]
    fn test_build_launch_command_for_web_application() {
        let dotnet_project = DotnetProject {
            assembly_name: Some("TestAssembly".to_string()),
            target_framework: "net6.0".to_string(),
            project_type: ProjectType::WebApplication,
            sdk_id: "Microsoft.NET.Sdk.Web".to_string(),
        };
        let executable_path =
            Path::new("/path/to/bin/Release/net6.0/linux-x64/publish/TestAssembly");
        let command = build_launch_command(&dotnet_project, executable_path);
        assert_eq!(
            command,
            "/path/to/bin/Release/net6.0/linux-x64/publish/TestAssembly --urls http://0.0.0.0:$PORT"
        );
    }

    #[test]
    fn test_build_launch_command_for_console_application() {
        let dotnet_project = DotnetProject {
            assembly_name: Some("TestAssembly".to_string()),
            target_framework: "net6.0".to_string(),
            project_type: ProjectType::ConsoleApplication,
            sdk_id: "Microsoft.NET.Sdk".to_string(),
        };
        let executable_path =
            Path::new("/path/to/bin/Release/net6.0/linux-x64/publish/TestAssembly");
        let command = build_launch_command(&dotnet_project, executable_path);
        assert_eq!(
            command,
            "/path/to/bin/Release/net6.0/linux-x64/publish/TestAssembly"
        );
    }
}
