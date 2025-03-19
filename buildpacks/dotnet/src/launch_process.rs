use crate::dotnet::project::ProjectType;
use crate::dotnet::solution::Solution;
use crate::Project;
use libcnb::data::launch::{Process, ProcessBuilder, ProcessType, ProcessTypeError};
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum LaunchProcessDetectionError {
    ProcessType(ProcessTypeError),
}

pub(crate) fn detect_solution_processes(
    solution: &Solution,
) -> Result<Vec<Process>, LaunchProcessDetectionError> {
    solution
        .projects
        .iter()
        .filter(|project| {
            matches!(
                project.project_type,
                ProjectType::ConsoleApplication
                    | ProjectType::WebApplication
                    | ProjectType::WorkerService
            )
        })
        .map(|project| project_launch_process(solution, project))
        .collect::<Result<_, _>>()
}

fn project_launch_process(
    solution: &Solution,
    project: &Project,
) -> Result<Process, LaunchProcessDetectionError> {
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

    project
        .assembly_name
        .parse::<ProcessType>()
        .map_err(LaunchProcessDetectionError::ProcessType)
        .map(|process_type| ProcessBuilder::new(process_type, ["bash", "-c", &command]).build())
}

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

fn project_executable_path(project: &Project) -> PathBuf {
    project
        .path
        .parent()
        .expect("Project file should always have a parent directory")
        .join("bin")
        .join("publish")
        .join(&project.assembly_name)
}
