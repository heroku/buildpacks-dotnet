use crate::dotnet_project::ProjectType;
use crate::dotnet_rid::RuntimeIdentifier;
use crate::dotnet_solution::DotnetSolution;
use libcnb::data::launch::{
    Process, ProcessBuilder, ProcessType, ProcessTypeError, WorkingDirectory,
};

#[derive(thiserror::Error, Debug)]
pub(crate) enum LaunchProcessDetectionError {
    #[error("Project has an invalid process type name: {0}")]
    ProcessType(ProcessTypeError),
}

pub(crate) fn detect_solution_processes(
    solution: &DotnetSolution,
    configuration: &str,
    rid: &RuntimeIdentifier,
) -> Result<Vec<Process>, LaunchProcessDetectionError> {
    solution
        .projects
        .iter()
        .filter(|project| {
            matches!(
                project.project_type,
                ProjectType::WebApplication | ProjectType::ConsoleApplication
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
                .join(rid.to_string())
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
