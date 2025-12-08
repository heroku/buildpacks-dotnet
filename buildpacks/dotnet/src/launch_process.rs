use crate::dotnet::project::ProjectType;
use crate::dotnet::solution::Solution;
use crate::{Project, utils};
use libcnb::data::launch::{Process, ProcessBuilder, ProcessType};
use libcnb::data::process_type;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProcessDetectionResult {
    Valid {
        relative_source: PathBuf,
        relative_artifact: PathBuf,
        process: Process,
    },
    Invalid {
        relative_source: PathBuf,
        relative_artifact: PathBuf,
    },
}

pub(crate) fn detect_solution_processes(
    app_dir: &Path,
    solution: &Solution,
) -> Vec<ProcessDetectionResult> {
    let has_single_web_app = solution
        .projects
        .iter()
        .filter(|p| p.project_type == ProjectType::WebApplication)
        .count()
        == 1;

    solution
        .projects
        .iter()
        .filter(|project| matches!(
            project.project_type,
            ProjectType::ConsoleApplication | ProjectType::WebApplication | ProjectType::WorkerService
        ))
        .map(|project| {
            let relative_source = project
                .path
                .strip_prefix(app_dir)
                .expect("Project path should be inside the app directory")
                .to_path_buf();

            let relative_artifact = relative_executable_path(app_dir, project);
            let absolute_artifact = app_dir.join(&relative_artifact);

            if !absolute_artifact.exists() {
                return ProcessDetectionResult::Invalid {
                    relative_source,
                    relative_artifact,
                };
            }

            let command = build_command(&relative_artifact, project.project_type);
            let process_type = project_process_type(project);
            let mut process = ProcessBuilder::new(process_type, ["bash", "-c", &command]).build();

            if has_single_web_app && project.project_type == ProjectType::WebApplication {
                process.r#type = process_type!("web");
                process.default = true;
            }

            ProcessDetectionResult::Valid {
                relative_source,
                relative_artifact,
                process,
            }
        })
        .collect()
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

/// Returns the (expected) relative executable path from the app directory
fn relative_executable_path(app_dir: &Path, project: &Project) -> PathBuf {
    project_executable_path(project)
        .strip_prefix(app_dir)
        .expect("Executable path should be inside the app directory")
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
    use libcnb::data::launch::WorkingDirectory;
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
    fn test_detect_solution_processes_single_web_app_valid() {
        let temp_dir = tempfile::tempdir().unwrap();
        let app_dir = temp_dir.path();

        let project = create_test_project(
            &format!("{}/bar/bar.csproj", app_dir.display()),
            "bar",
            ProjectType::WebApplication,
        );

        create_test_artifact(app_dir, &project);

        let solution = Solution {
            path: app_dir.join("foo.sln"),
            projects: vec![project],
        };

        let results = detect_solution_processes(app_dir, &solution);
        assert_eq!(results.len(), 1);

        assert_eq!(
            results[0],
            ProcessDetectionResult::Valid {
                relative_source: PathBuf::from("bar/bar.csproj"),
                relative_artifact: PathBuf::from("bar/bin/publish/bar"),
                process: Process {
                    r#type: process_type!("web"),
                    command: vec![
                        "bash".to_string(),
                        "-c".to_string(),
                        "cd bar/bin/publish; ./bar --urls http://*:$PORT".to_string(),
                    ],
                    args: vec![],
                    default: true,
                    working_directory: WorkingDirectory::App,
                }
            }
        );
    }

    #[test]
    fn test_detect_solution_processes_multiple_web_apps_valid() {
        let temp_dir = tempfile::tempdir().unwrap();
        let app_dir = temp_dir.path();

        let project1 = create_test_project(
            &format!("{}/bar/bar.csproj", app_dir.display()),
            "bar",
            ProjectType::WebApplication,
        );
        let project2 = create_test_project(
            &format!("{}/baz/baz.csproj", app_dir.display()),
            "baz",
            ProjectType::WebApplication,
        );

        create_test_artifact(app_dir, &project1);
        create_test_artifact(app_dir, &project2);

        let solution = Solution {
            path: app_dir.join("foo.sln"),
            projects: vec![project1, project2],
        };

        let results = detect_solution_processes(app_dir, &solution);
        assert_eq!(results.len(), 2);

        let result = &results[0];
        assert_matches!(result, ProcessDetectionResult::Valid { process, .. } if process.r#type == process_type!("bar") && !process.default);
        let result = &results[1];
        assert_matches!(result, ProcessDetectionResult::Valid { process, .. } if process.r#type == process_type!("baz") && !process.default);
    }

    #[test]
    fn test_detect_solution_processes_mixed_valid_and_invalid() {
        let temp_dir = tempfile::tempdir().unwrap();
        let app_dir = temp_dir.path();

        let valid_project = create_test_project(
            &format!("{}/Backend/Backend.csproj", app_dir.display()),
            "Backend",
            ProjectType::WebApplication,
        );
        let invalid_project = create_test_project(
            &format!("{}/jobs/worker.cs", app_dir.display()),
            "worker",
            ProjectType::ConsoleApplication,
        );

        create_test_artifact(app_dir, &valid_project);

        let solution = Solution {
            path: app_dir.join("solution.sln"),
            projects: vec![valid_project, invalid_project],
        };

        let results = detect_solution_processes(app_dir, &solution);
        assert_eq!(results.len(), 2);

        let result = &results[0];
        assert_matches!(
            result,
            ProcessDetectionResult::Valid {
                relative_source,
                relative_artifact,
                process,
            } if relative_source == &PathBuf::from("Backend/Backend.csproj")
                && relative_artifact == &PathBuf::from("Backend/bin/publish/Backend")
                && process.r#type == process_type!("web")
                && process.default
        );

        let result = &results[1];
        assert_matches!(
            result,
            ProcessDetectionResult::Invalid {
                relative_source,
                relative_artifact,
            } if relative_source == &PathBuf::from("jobs/worker.cs")
                && relative_artifact == &PathBuf::from("jobs/bin/publish/worker")
        );
    }

    fn create_test_artifact(_app_dir: &Path, project: &Project) -> std::path::PathBuf {
        let artifact_path = project_executable_path(project);

        fs_err::create_dir_all(artifact_path.parent().unwrap()).unwrap();
        fs_err::write(&artifact_path, b"").unwrap();
        artifact_path
    }

    #[test]
    fn test_relative_executable_path() {
        let app_dir = Path::new("/tmp");
        let project = create_test_project(
            "/tmp/project/project.csproj",
            "TestApp",
            ProjectType::ConsoleApplication,
        );

        assert_eq!(
            relative_executable_path(app_dir, &project),
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

    #[test]
    fn test_detect_solution_processes_filters_non_executable_projects() {
        let app_dir = Path::new("/tmp");
        let solution = Solution {
            path: PathBuf::from("/tmp/foo.sln"),
            projects: vec![
                create_test_project("/tmp/lib/lib.csproj", "lib", ProjectType::Unknown),
                create_test_project("/tmp/bar/bar.csproj", "bar", ProjectType::WebApplication),
            ],
        };

        let results = detect_solution_processes(app_dir, &solution);
        assert_eq!(results.len(), 1);

        let result = &results[0];
        assert_matches!(result, ProcessDetectionResult::Invalid { relative_source, ..} if relative_source == "bar/bar.csproj");
    }
}
