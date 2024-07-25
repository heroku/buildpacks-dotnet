use crate::dotnet::project::ProjectType;
use crate::dotnet::runtime_identifier::RuntimeIdentifier;
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
    configuration: &str,
    runtime_identifier: &RuntimeIdentifier,
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
                .join(configuration)
                .join(&project.target_framework)
                .join(runtime_identifier.to_string())
                .join("publish")
                .join(&project.assembly_name);
            let mut command = format!("{}", executable_path.to_string_lossy());

            if project.project_type == ProjectType::WebApplication {
                command.push_str(" --urls http://0.0.0.0:$PORT");
            }

            project
                .assembly_name
                .parse::<ProcessType>()
                .map_err(LaunchProcessDetectionError::ProcessType)
                .map(|process_type| {
                    ProcessBuilder::new(process_type, ["bash", "-c", &command])
                        .working_directory(WorkingDirectory::Directory(
                            executable_path
                                .parent()
                                .expect("Executable should always have a parent directory")
                                .to_path_buf(),
                        ))
                        .build()
                })
        })
        .collect::<Result<_, _>>()
}
