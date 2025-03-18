use crate::dotnet::project::ProjectType;
use crate::dotnet::solution::Solution;
use libcnb::data::launch::{Process, ProcessBuilder, ProcessType, ProcessTypeError};

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

            let mut command = format!(
                "cd {}; ./{}",
                shell_words::quote(
                    &relative_executable_path
                        .parent()
                        .expect("Path to always have a parent directory")
                        .to_string_lossy()
                ),
                shell_words::quote(
                    &relative_executable_path
                        .file_name()
                        .expect("Path to never terminate in `..`")
                        .to_string_lossy()
                )
            );

            if project.project_type == ProjectType::WebApplication {
                command.push_str(" --urls http://*:$PORT");
            }

            sanitize_process_type_name(&project.assembly_name)
                .parse::<ProcessType>()
                .map_err(LaunchProcessDetectionError::ProcessType)
                .map(|process_type| {
                    ProcessBuilder::new(process_type, ["bash", "-c", &command]).build()
                })
        })
        .collect::<Result<_, _>>()
}

fn sanitize_process_type_name(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
            sanitize_process_type_name("Special chars: !@#$%^&*()"),
            "Specialchars"
        );
        assert_eq!(
            sanitize_process_type_name("Mixed: aBc123.xyz_-.!@#"),
            "MixedaBc123.xyz_-."
        );
        assert_eq!(
            sanitize_process_type_name("Unicode: 日本語123"),
            "Unicode123"
        );
    }
}
