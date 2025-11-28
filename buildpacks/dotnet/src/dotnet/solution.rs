use crate::dotnet::project::{self, Project};
use crate::dotnet::slnx;
use regex::Regex;
use std::io::{self};
use std::path::{Path, PathBuf};

pub(crate) struct Solution {
    pub(crate) path: PathBuf,
    pub(crate) projects: Vec<Project>,
}

impl Solution {
    pub(crate) fn load_from_path(path: &Path) -> Result<Self, LoadError> {
        let contents = fs_err::read_to_string(path).map_err(LoadError::ReadSolutionFile)?;
        let project_paths = if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "slnx")
        {
            slnx::extract_project_paths(&contents).map_err(LoadError::SlnxParseError)?
        } else {
            extract_project_references(&contents)
        };

        Ok(Self {
            path: path.to_path_buf(),
            projects: project_paths
                .into_iter()
                .filter_map(|project_path| path.parent().map(|dir| dir.join(&project_path)))
                .map(try_load_project)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub(crate) fn ephemeral(project: Project) -> Self {
        Self {
            path: project.path.clone(),
            projects: vec![project],
        }
    }
}

fn try_load_project(path: PathBuf) -> Result<Project, LoadError> {
    path.try_exists()
        .map_err(|error| LoadError::LoadProject(project::LoadError::ReadProjectFile(error)))
        .and_then(|exists| {
            if exists {
                Project::load_from_path(&path).map_err(LoadError::LoadProject)
            } else {
                Err(LoadError::ProjectNotFound(path))
            }
        })
}

#[derive(Debug)]
pub(crate) enum LoadError {
    ReadSolutionFile(io::Error),
    ProjectNotFound(PathBuf),
    LoadProject(project::LoadError),
    SlnxParseError(quick_xml::DeError),
}

fn extract_project_references(contents: &str) -> Vec<String> {
    let project_line_regex =
        Regex::new(r#"Project\("\{[^}]+\}"\) = "[^"]+", "([^"]+\.[^"]+)", "\{[^}]+\}""#)
            .expect("regex to be valid");
    contents
        .lines()
        .filter_map(|line| {
            project_line_regex
                .captures(line)
                .and_then(|captures| captures.get(1))
                .map(|project_path| project_path.as_str().replace('\\', "/"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::ErrorKind;

    const SIMPLE_PROJECT_CONTENT: &str = r#"
        <Project Sdk="Microsoft.NET.Sdk">
            <PropertyGroup>
                <TargetFramework>net6.0</TargetFramework>
            </PropertyGroup>
        </Project>"#;

    const SOLUTION_WITH_TWO_PROJECTS: &str = r#"
        Microsoft Visual Studio Solution File, Format Version 12.00
        # Visual Studio Version 16
        VisualStudioVersion = 16.0.28729.10
        MinimumVisualStudioVersion = 10.0.40219.1
        Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "Project1", "Project1\Project1.csproj", "{8C28B63A-F94D-4A0B-A2B0-6DC6E1B88264}"
        EndProject
        Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "Project2", "Project2\Project2.csproj", "{FEA4E2C3-9F8E-4A2C-88C9-1E6E41F8B9AD}"
        EndProject
        Global
        GlobalSection(SolutionConfigurationPlatforms) = preSolution
            Debug|Any CPU = Debug|Any CPU
            Release|Any CPU = Release|Any CPU
        EndGlobalSection
        EndGlobal
        "#;

    fn create_test_project(temp_dir: &tempfile::TempDir, project_name: &str) -> PathBuf {
        let project_dir = temp_dir.path().join(project_name);
        fs::create_dir(&project_dir).unwrap();
        let project_path = project_dir.join(format!("{project_name}.csproj"));
        fs::write(&project_path, SIMPLE_PROJECT_CONTENT).unwrap();
        project_path
    }

    #[test]
    fn test_try_load_project_with_invalid_path() {
        // Create an invalid path with null bytes which will cause try_exists() to fail
        let invalid_path = PathBuf::from("some\0file.csproj");

        let result = try_load_project(invalid_path);
        assert!(
            matches!(result, Err(LoadError::LoadProject(project::LoadError::ReadProjectFile(error))) if error.kind() == ErrorKind::InvalidInput)
        );
    }

    #[test]
    fn test_extract_project_references_should_find_all_projects_in_solution() {
        let project_references = extract_project_references(SOLUTION_WITH_TWO_PROJECTS);

        assert_eq!(project_references.len(), 2);
        assert_eq!(project_references[0], "Project1/Project1.csproj");
        assert_eq!(project_references[1], "Project2/Project2.csproj");
    }

    #[test]
    fn test_extract_project_references_should_return_empty_vec_for_solution_with_no_projects() {
        let solution_content = r"
        Microsoft Visual Studio Solution File, Format Version 12.00
        # Visual Studio Version 16
        VisualStudioVersion = 16.0.28729.10
        MinimumVisualStudioVersion = 10.0.40219.1
        Global
        GlobalSection(SolutionConfigurationPlatforms) = preSolution
            Debug|Any CPU = Debug|Any CPU
            Release|Any CPU = Release|Any CPU
        EndGlobalSection
        EndGlobal
        ";

        let project_references = extract_project_references(solution_content);
        assert_eq!(project_references.len(), 0);
    }

    #[test]
    fn test_extract_project_references_should_ignore_solution_folders() {
        let solution_content = r#"
        Microsoft Visual Studio Solution File, Format Version 12.00
        # Visual Studio Version 16
        VisualStudioVersion = 16.0.28729.10
        MinimumVisualStudioVersion = 10.0.40219.1
        Project("{66A26720-8FB5-11D2-AA7E-00C04F688DDE}") = "SolutionFolder", "SolutionFolder", "{66A26720-8FB5-11D2-AA7E-00C04F688DDE}"
        Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "NestedProject", "SolutionFolder\NestedProject\NestedProject.csproj", "{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}"
        EndProject
        EndProject
        Global
        GlobalSection(SolutionConfigurationPlatforms) = preSolution
            Debug|Any CPU = Debug|Any CPU
            Release|Any CPU = Release|Any CPU
        EndGlobalSection
        EndGlobal
        "#;

        let project_references = extract_project_references(solution_content);
        assert_eq!(project_references.len(), 1);
        assert_eq!(
            project_references[0],
            "SolutionFolder/NestedProject/NestedProject.csproj"
        );
    }

    #[test]
    fn test_extract_project_references_should_ignore_solution_items() {
        let solution_content = r#"
        Microsoft Visual Studio Solution File, Format Version 12.00
        # Visual Studio Version 16
        VisualStudioVersion = 16.0.28729.10
        MinimumVisualStudioVersion = 10.0.40219.1
        Project("{66A26720-8FB5-11D2-AA7E-00C04F688DDE}") = "Solution Items", "Solution Items", "{8B41D0E4-BA13-4EF0-B103-20302B2B9F9A}"
        ProjectSection(SolutionItems) = preProject
            Readme.md = Readme.md
            Config.xml = Config.xml
        EndProjectSection
        EndProject
        Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "ProjectWithParams", "ProjectWithParams\ProjectWithParams.csproj", "{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}"
        GlobalSection(ProjectConfigurationPlatforms) = postSolution
            {FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
            {FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}.Debug|Any CPU.Build.0 = Debug|Any CPU
        EndGlobalSection
        EndProject
        Global
        GlobalSection(SolutionConfigurationPlatforms) = preSolution
            Debug|Any CPU = Debug|Any CPU
            Release|Any CPU = Release|Any CPU
        EndGlobalSection
        EndGlobal
        "#;

        let project_references = extract_project_references(solution_content);
        assert_eq!(project_references.len(), 1);
        assert_eq!(
            project_references[0],
            "ProjectWithParams/ProjectWithParams.csproj"
        );
    }

    #[test]
    fn test_load_from_path_should_load_all_projects_in_solution() {
        let temp_dir = tempfile::tempdir().unwrap();
        let solution_path = temp_dir.path().join("test.sln");

        fs::write(&solution_path, SOLUTION_WITH_TWO_PROJECTS).unwrap();
        let project1_path = create_test_project(&temp_dir, "Project1");
        let project2_path = create_test_project(&temp_dir, "Project2");

        let solution = Solution::load_from_path(&solution_path).unwrap();

        assert_eq!(solution.path, solution_path);
        assert_eq!(solution.projects.len(), 2);
        assert_eq!(solution.projects[0].path, project1_path);
        assert_eq!(solution.projects[1].path, project2_path);
    }

    #[test]
    fn test_load_from_path_should_return_error_when_solution_file_does_not_exist() {
        let temp_dir = tempfile::tempdir().unwrap();
        let non_existent_path = temp_dir.path().join("nonexistent.sln");

        let result = Solution::load_from_path(&non_existent_path);
        assert!(
            matches!(result, Err(LoadError::ReadSolutionFile(error)) if error.kind() == ErrorKind::NotFound)
        );
    }

    #[test]
    fn test_load_from_path_should_return_error_when_project_file_does_not_exist() {
        let temp_dir = tempfile::tempdir().unwrap();
        let solution_path = temp_dir.path().join("test.sln");
        let missing_project_path = temp_dir.path().join("Project1").join("Project1.csproj");

        let solution_content = r#"
        Microsoft Visual Studio Solution File, Format Version 12.00
        Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "Project1", "Project1\Project1.csproj", "{8C28B63A-F94D-4A0B-A2B0-6DC6E1B88264}"
        EndProject
        "#;
        fs::write(&solution_path, solution_content).unwrap();

        let result = Solution::load_from_path(&solution_path);
        assert!(
            matches!(result, Err(LoadError::ProjectNotFound(path)) if path == missing_project_path)
        );
    }

    #[test]
    fn test_ephemeral_solution_should_contain_single_project() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = create_test_project(&temp_dir, "test");
        let project = Project::load_from_path(&project_path).unwrap();
        let solution = Solution::ephemeral(project);

        assert_eq!(solution.path, project_path);
        assert_eq!(solution.projects.len(), 1);
        assert_eq!(solution.projects[0].path, project_path);
    }

    #[test]
    fn test_load_from_path_should_load_all_projects_in_slnx_solution() {
        let temp_dir = tempfile::tempdir().unwrap();
        let solution_path = temp_dir.path().join("test.slnx");

        let slnx_content = r#"
        <Solution>
          <Project Path="Project1\Project1.csproj" />
          <Project Path="Project2\Project2.csproj" />
        </Solution>
        "#;
        fs::write(&solution_path, slnx_content).unwrap();
        let project1_path = create_test_project(&temp_dir, "Project1");
        let project2_path = create_test_project(&temp_dir, "Project2");

        let solution = Solution::load_from_path(&solution_path).unwrap();

        assert_eq!(solution.path, solution_path);
        assert_eq!(solution.projects.len(), 2);
        assert_eq!(solution.projects[0].path, project1_path);
        assert_eq!(solution.projects[1].path, project2_path);
    }

    #[test]
    fn test_load_from_path_should_return_error_when_slnx_file_has_malformed_xml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let solution_path = temp_dir.path().join("test.slnx");

        let malformed_slnx = r#"
        <Solution>
          <Project Path="Project1\Project1.csproj" />
        "#;
        fs::write(&solution_path, malformed_slnx).unwrap();

        let result = Solution::load_from_path(&solution_path);
        assert!(matches!(result, Err(LoadError::SlnxParseError(_)) if true));
    }
}
