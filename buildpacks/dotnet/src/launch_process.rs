use crate::dotnet::project::ProjectType;
use crate::dotnet::solution::Solution;
use crate::Project;
use libcnb::data::launch::{Process, ProcessBuilder, ProcessType};
use std::path::PathBuf;

/// Detects processes in a solution's projects
pub(crate) fn detect_solution_processes(solution: &Solution) -> Vec<Process> {
    solution
        .projects
        .iter()
        .filter_map(|project| project_launch_process(solution, project))
        .collect()
}

/// Determines if a project should have a launchable process and constructs it
fn project_launch_process(solution: &Solution, project: &Project) -> Option<Process> {
    if !matches!(
        project.project_type,
        ProjectType::ConsoleApplication | ProjectType::WebApplication | ProjectType::WorkerService
    ) {
        return None;
    }
    let relative_executable_path = relative_executable_path(solution, project);

    let mut command = format!(
        "cd {}; ./{}",
        relative_executable_path
            .parent()
            .expect("Path to always have a parent directory")
            .display(),
        relative_executable_path
            .file_name()
            .expect("Path to never terminate in `..`")
            .to_string_lossy()
    );

    if project.project_type == ProjectType::WebApplication {
        command.push_str(" --urls http://*:$PORT");
    }

    Some(ProcessBuilder::new(project_process_type(project), ["bash", "-c", &command]).build())
}

/// Returns a sanitized process type name, ensuring it is always valid
fn project_process_type(project: &Project) -> ProcessType {
    sanitize_process_type_name(&project.assembly_name)
        .parse::<ProcessType>()
        .expect("Sanitized process type name should always be valid")
}

/// Returns the (expected) relative executable path from the solution's parent directory
fn relative_executable_path(solution: &Solution, project: &Project) -> PathBuf {
    project_executable_path(project)
        .strip_prefix(
            solution
                .path
                .parent()
                .expect("Solution path to have a parent"),
        )
        .expect("Project to be nested in solution parent directory")
        .to_path_buf()
}

/// Returns the (expected) absolute path to the project's compiled executable
fn project_executable_path(project: &Project) -> PathBuf {
    project
        .path
        .parent()
        .expect("Project file should always have a parent directory")
        .join("bin")
        .join("publish")
        .join(&project.assembly_name)
}

/// Sanitizes a process type name to only contain allowed characters
fn sanitize_process_type_name(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use libcnb::data::launch::{Process, WorkingDirectory};
    use libcnb::data::process_type;
    use std::path::PathBuf;

    #[test]
    fn test_detect_solution_processes_web_app() {
        let solution = Solution {
            path: PathBuf::from("/tmp/foo.sln"),
            projects: vec![Project {
                path: PathBuf::from("/tmp/bar/bar.csproj"),
                target_framework: "net9.0".to_string(),
                project_type: ProjectType::WebApplication,
                assembly_name: "bar".to_string(),
            }],
        };

        let expected_processes = vec![Process {
            r#type: process_type!("bar"),
            command: vec![
                "bash".to_string(),
                "-c".to_string(),
                "cd bar/bin/publish; ./bar --urls http://*:$PORT".to_string(),
            ],
            args: vec![],
            default: false,
            working_directory: WorkingDirectory::App,
        }];

        assert_eq!(detect_solution_processes(&solution), expected_processes);
    }

    #[test]
    fn test_detect_solution_processes_console_app() {
        let solution = Solution {
            path: PathBuf::from("/tmp/foo.sln"),
            projects: vec![Project {
                path: PathBuf::from("/tmp/bar/bar.csproj"),
                target_framework: "net9.0".to_string(),
                project_type: ProjectType::ConsoleApplication,
                assembly_name: "bar".to_string(),
            }],
        };

        let expected_processes = vec![Process {
            r#type: process_type!("bar"),
            command: vec![
                "bash".to_string(),
                "-c".to_string(),
                "cd bar/bin/publish; ./bar".to_string(),
            ],
            args: vec![],
            default: false,
            working_directory: WorkingDirectory::App,
        }];

        assert_eq!(detect_solution_processes(&solution), expected_processes);
    }

    #[test]
    fn test_project_launch_process_non_executable() {
        let solution = Solution {
            path: PathBuf::from("/tmp/foo.sln"),
            projects: vec![Project {
                path: PathBuf::from("/tmp/bar/bar.csproj"),
                target_framework: "net9.0".to_string(),
                project_type: ProjectType::Unknown,
                assembly_name: "bar".to_string(),
            }],
        };

        assert!(detect_solution_processes(&solution).is_empty());
    }

    #[test]
    fn test_project_executable_path() {
        let project = Project {
            path: PathBuf::from("/tmp/project/project.csproj"),
            target_framework: "net9.0".to_string(),
            project_type: ProjectType::ConsoleApplication,
            assembly_name: "TestApp".to_string(),
        };

        assert_eq!(
            project_executable_path(&project),
            PathBuf::from("/tmp/project/bin/publish/TestApp")
        );
    }

    #[test]
    fn test_relative_executable_path() {
        let solution = Solution {
            path: PathBuf::from("/tmp/solution.sln"),
            projects: vec![],
        };

        let project = Project {
            path: PathBuf::from("/tmp/project/project.csproj"),
            target_framework: "net9.0".to_string(),
            project_type: ProjectType::ConsoleApplication,
            assembly_name: "TestApp".to_string(),
        };

        assert_eq!(
            relative_executable_path(&solution, &project),
            PathBuf::from("project/bin/publish/TestApp")
        );
    }

    #[test]
    fn test_sanitize_process_type_name() {
        assert_eq!(
            sanitize_process_type_name("Hello, world! 123"),
            "Helloworld123"
        );
        assert_eq!(
            sanitize_process_type_name("This_is-a.test.123.abc"),
            "This_is-a.test.123.abc"
        );
        assert_eq!(
            sanitize_process_type_name("Special chars: !@#$%+^&*()"),
            "Specialchars"
        );
        assert_eq!(
            sanitize_process_type_name("Mixed: aBc123.xyz_-!@#"),
            "MixedaBc123.xyz_-"
        );
        assert_eq!(
            sanitize_process_type_name("Unicode: 日本語123"),
            "Unicode123"
        );
    }
}
