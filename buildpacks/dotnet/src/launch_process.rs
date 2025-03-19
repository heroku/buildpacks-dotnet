use crate::dotnet::project::ProjectType;
use crate::dotnet::solution::Solution;
use crate::Project;
use libcnb::data::launch::{Process, ProcessBuilder, ProcessType, ProcessTypeError};
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum LaunchProcessDetectionError {
    ProcessType(ProcessTypeError),
}

/// Detects processes in a solution's projects
pub(crate) fn detect_solution_processes(
    solution: &Solution,
) -> Result<Vec<Process>, LaunchProcessDetectionError> {
    solution
        .projects
        .iter()
        .filter_map(|project| project_launch_process(solution, project))
        .collect::<Result<_, _>>()
}

/// Determines if a project should have a launchable process and constructs it
fn project_launch_process(
    solution: &Solution,
    project: &Project,
) -> Option<Result<Process, LaunchProcessDetectionError>> {
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

    Some(
        project_process_type(project).map(|process_type| {
            ProcessBuilder::new(process_type, ["bash", "-c", &command]).build()
        }),
    )
}

fn project_process_type(project: &Project) -> Result<ProcessType, LaunchProcessDetectionError> {
    project
        .assembly_name
        .parse::<ProcessType>()
        .map_err(LaunchProcessDetectionError::ProcessType)
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
