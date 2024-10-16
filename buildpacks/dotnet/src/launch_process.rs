use crate::dotnet::project::ProjectType;
use crate::dotnet::solution::Solution;
use libcnb::data::launch::{
    Process, ProcessBuilder, ProcessType, ProcessTypeError, WorkingDirectory,
};

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
        .map(|project| {
            let executable_path = project
                .path
                .parent()
                .expect("Project file should always have a parent directory")
                .join("bin")
                .join("publish")
                .join(&project.assembly_name);

            let relative_executable_path = executable_path
                .strip_prefix(
                    solution
                        .path
                        .parent()
                        .expect("Solution path to have a parent"),
                )
                .expect("Project to be nested in solution parent directory");

            let mut command = relative_executable_path
                .file_name()
                .expect("Executable path to never terminate in `..`")
                .to_string_lossy()
                .to_string();

            if project.project_type == ProjectType::WebApplication {
                command.push_str(" --urls http://0.0.0.0:$PORT");
            }

            project
                .assembly_name
                .parse::<ProcessType>()
                .map_err(LaunchProcessDetectionError::ProcessType)
                .map(|process_type| {
                    ProcessBuilder::new(process_type, ["bash", "-c", &format!("./{}", &command)])
                        .working_directory(WorkingDirectory::Directory(
                            relative_executable_path
                                .parent()
                                .expect("Executable path to always have a parent directory")
                                .to_path_buf(),
                        ))
                        .build()
                })
        })
        .collect::<Result<_, _>>()
}
