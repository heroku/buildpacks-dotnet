use crate::dotnet_project::{self, DotnetProject, ProjectType};
use crate::{dotnet_rid, dotnet_solution, DotnetFile};
use libcnb::data::launch::{
    Process, ProcessBuilder, ProcessType, ProcessTypeError, WorkingDirectory,
};
use libherokubuildpack::log::log_info;
use std::io;

#[derive(Debug, thiserror::Error)]
pub(crate) enum LaunchProcessError {
    #[error(transparent)]
    DotnetProject(#[from] dotnet_project::ParseError),
    #[error("Error parsing target framework: {0}")]
    ParseSolutionFile(io::Error),
    #[error("Invalid CNB process name: {0}")]
    ProcessName(#[from] ProcessTypeError),
}

impl TryFrom<&DotnetFile> for Vec<Process> {
    type Error = LaunchProcessError;

    fn try_from(value: &DotnetFile) -> Result<Self, Self::Error> {
        match value {
            DotnetFile::Solution(path) => {
                log_info("Detecting solution executables");
                let mut project_processes = vec![];
                for project_path in dotnet_solution::project_file_paths(path)
                    .map_err(LaunchProcessError::ParseSolutionFile)?
                {
                    project_processes.push(Self::try_from(&DotnetFile::Project(project_path))?);
                }
                Ok(project_processes.into_iter().flatten().collect())
            }
            DotnetFile::Project(project_path) => {
                let dotnet_project = DotnetProject::try_from(project_path.as_path())?;
                if matches!(
                    dotnet_project.project_type,
                    |ProjectType::ConsoleApplication| ProjectType::WebApplication
                        | ProjectType::RazorApplication
                        | ProjectType::Worker
                ) {
                    let executable_name = match &dotnet_project.assembly_name {
                        Some(name) if !name.is_empty() => name.clone(),
                        _ => project_path
                            .file_stem()
                            .expect("project file path to have a file name")
                            .to_string_lossy()
                            .to_string(),
                    };

                    let executable_path = project_path
                        .parent()
                        .expect("Project file will always have a parent directory")
                        .join("bin")
                        .join("Release")
                        .join(dotnet_project.target_framework)
                        .join(dotnet_rid::get_runtime_identifier().to_string())
                        .join("publish")
                        .join(&executable_name);

                    // TODO: We have to cd to the working directory (as libcnb.rs doesn't currently do it for us <https://github.com/heroku/libcnb.rs/pull/831>).
                    // Refactor this when libcnb.rs correctly sets the configured working directory.
                    let executable_working_dir = executable_path
                        .parent()
                        .expect("Executable to have a parent directory")
                        .to_path_buf();

                    let mut command = format!(
                        "cd {}; {}",
                        executable_working_dir.to_string_lossy(),
                        executable_path.to_string_lossy()
                    );

                    match dotnet_project.project_type {
                        ProjectType::WebApplication
                        | ProjectType::RazorApplication
                        | ProjectType::BlazorWebAssembly => {
                            log_info(format!(
                                "Detected web process type \"{}\" ({})",
                                executable_name,
                                executable_path.to_string_lossy()
                            ));
                            command.push_str(" --urls http://0.0.0.0:$PORT");
                        }
                        _ => {
                            log_info(format!(
                                "Detected console process type \"{}\" ({:?})",
                                executable_name,
                                executable_path.to_string_lossy()
                            ));
                        }
                    };

                    Ok(vec![ProcessBuilder::new(
                        executable_name.parse::<ProcessType>()?,
                        ["bash", "-c", &command],
                    )
                    // TODO: libcnb.rs doesn't honor this setting, and `working-dir` will always be the default `/workspace`.
                    // Remove this comment when libcnb.rs correctly sets the configured working directory.
                    .working_directory(WorkingDirectory::Directory(executable_working_dir))
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
        }
    }
}
