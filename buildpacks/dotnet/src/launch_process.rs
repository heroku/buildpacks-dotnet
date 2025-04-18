use crate::dotnet::project::ProjectType;
use crate::dotnet::solution::Solution;
use crate::{Project, utils};
use libcnb::data::launch::{Process, ProcessBuilder, ProcessType};
use libcnb::data::process_type;
use std::path::{Path, PathBuf};

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

    let command = build_command(&relative_executable_path, project.project_type);

    let process_type = match project.project_type {
        // If project is a web application, and there's only one web application in the solution,
        // set the process type to `web`.
        ProjectType::WebApplication
            if solution
                .projects
                .iter()
                .filter(|p| p.project_type == ProjectType::WebApplication)
                .count()
                == 1 =>
        {
            process_type!("web")
        }
        _ => project_process_type(project),
    };

    Some(ProcessBuilder::new(process_type, ["bash", "-c", &command]).build())
}

/// Constructs the shell command for launching the process
fn build_command(relative_executable_path: &Path, project_type: ProjectType) -> String {
    let parent_dir = relative_executable_path
        .parent()
        .expect("Executable path should always have a parent directory")
        .to_str()
        .expect("Path should be valid UTF-8");

    let file_name = relative_executable_path
        .file_name()
        .expect("Executable path should always have a file name")
        .to_str()
        .expect("Path should be valid UTF-8");

    let mut command = format!(
        "cd {}; ./{}",
        shell_words::quote(parent_dir),
        shell_words::quote(file_name)
    );

    if project_type == ProjectType::WebApplication {
        command.push_str(" --urls http://*:$PORT");
    }

    command
}

/// Returns a sanitized process type name, ensuring it is always valid
fn project_process_type(project: &Project) -> ProcessType {
    utils::to_rfc1123_label(&project.assembly_name)
        .expect("Assembly name to include at least one character compatible with the RFC 1123 DNS label spec")
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
                .expect("Solution file should be in a directory"),
        )
        .expect("Executable path should inside the solution's directory")
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

#[cfg(test)]
mod tests {
    use super::*;
    use libcnb::data::launch::{Process, WorkingDirectory};
    use libcnb::data::process_type;
    use std::path::PathBuf;

    fn create_test_project(path: &str, assembly_name: &str, project_type: ProjectType) -> Project {
        Project {
            path: PathBuf::from(path),
            target_framework: "net9.0".to_string(),
            project_type,
            assembly_name: assembly_name.to_string(),
        }
    }

    #[test]
    fn test_detect_solution_processes_single_web_app() {
        let solution = Solution {
            path: PathBuf::from("/tmp/foo.sln"),
            projects: vec![create_test_project(
                "/tmp/bar/bar.csproj",
                "bar",
                ProjectType::WebApplication,
            )],
        };

        let expected_processes = vec![Process {
            r#type: process_type!("web"),
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
    fn test_detect_solution_processes_multiple_web_apps() {
        let solution = Solution {
            path: PathBuf::from("/tmp/foo.sln"),
            projects: vec![
                create_test_project("/tmp/bar/bar.csproj", "bar", ProjectType::WebApplication),
                create_test_project("/tmp/baz/baz.csproj", "baz", ProjectType::WebApplication),
            ],
        };
        assert_eq!(
            detect_solution_processes(&solution)
                .iter()
                .map(|process| process.r#type.clone())
                .collect::<Vec<ProcessType>>(),
            vec![process_type!("bar"), process_type!("baz")]
        );
    }

    #[test]
    fn test_detect_solution_processes_single_web_app_and_console_app() {
        let solution = Solution {
            path: PathBuf::from("/tmp/foo.sln"),
            projects: vec![
                create_test_project("/tmp/qux/qux.csproj", "qux", ProjectType::Unknown),
                create_test_project("/tmp/bar/bar.csproj", "bar", ProjectType::WebApplication),
                create_test_project(
                    "/tmp/baz/baz.csproj",
                    "baz",
                    ProjectType::ConsoleApplication,
                ),
            ],
        };
        assert_eq!(
            detect_solution_processes(&solution)
                .iter()
                .map(|process| process.r#type.clone())
                .collect::<Vec<ProcessType>>(),
            vec![process_type!("web"), process_type!("baz")]
        );
    }

    #[test]
    fn test_detect_solution_processes_with_spaces() {
        let solution = Solution {
            path: PathBuf::from("/tmp/My Solution With Spaces.sln"),
            projects: vec![create_test_project(
                "/tmp/My Project With Spaces/project.csproj",
                "My App",
                ProjectType::ConsoleApplication,
            )],
        };

        let expected_processes = vec![Process {
            r#type: process_type!("my-app"),
            command: vec![
                "bash".to_string(),
                "-c".to_string(),
                "cd 'My Project With Spaces/bin/publish'; ./'My App'".to_string(),
            ],
            args: vec![],
            default: false,
            working_directory: WorkingDirectory::App,
        }];

        assert_eq!(detect_solution_processes(&solution), expected_processes);
    }

    #[test]
    fn test_relative_executable_path() {
        let solution = Solution {
            path: PathBuf::from("/tmp/solution.sln"),
            projects: vec![],
        };

        let project = create_test_project(
            "/tmp/project/project.csproj",
            "TestApp",
            ProjectType::ConsoleApplication,
        );

        assert_eq!(
            relative_executable_path(&solution, &project),
            PathBuf::from("project/bin/publish/TestApp")
        );
    }

    #[test]
    fn test_project_executable_path() {
        let project = create_test_project(
            "/tmp/project/project.csproj",
            "TestApp",
            ProjectType::ConsoleApplication,
        );

        assert_eq!(
            project_executable_path(&project),
            PathBuf::from("/tmp/project/bin/publish/TestApp")
        );
    }

    #[test]
    fn test_build_command_with_spaces() {
        let executable_path = PathBuf::from("some/project with spaces/bin/publish/My App");

        assert_eq!(
            build_command(&executable_path, ProjectType::ConsoleApplication),
            "cd 'some/project with spaces/bin/publish'; ./'My App'"
        );

        assert_eq!(
            build_command(&executable_path, ProjectType::WebApplication),
            "cd 'some/project with spaces/bin/publish'; ./'My App' --urls http://*:$PORT"
        );
    }

    #[test]
    fn test_build_command_with_special_chars() {
        let executable_path =
            PathBuf::from("some/project with #special$chars/bin/publish/My-App+v1.2_Release!");

        assert_eq!(
            build_command(&executable_path, ProjectType::ConsoleApplication),
            "cd 'some/project with #special$chars/bin/publish'; ./My-App+v1.2_Release!"
        );
    }
}
