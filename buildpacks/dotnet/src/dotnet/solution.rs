use crate::dotnet::project::{self, Project};
use regex::Regex;
use std::fs::{self};
use std::io::{self};
use std::path::{Path, PathBuf};

pub(crate) struct Solution {
    pub(crate) path: PathBuf,
    pub(crate) projects: Vec<Project>,
}

impl Solution {
    pub(crate) fn load_from_path(path: &Path) -> Result<Self, LoadError> {
        Ok(Self {
            path: path.to_path_buf(),
            projects: extract_project_references(
                &fs::read_to_string(path).map_err(LoadError::ReadSolutionFile)?,
            )
            .into_iter()
            .filter_map(|project_path| {
                path.parent().map(|dir| {
                    Project::load_from_path(&dir.join(project_path)).map_err(LoadError::LoadProject)
                })
            })
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

#[derive(Debug)]
pub(crate) enum LoadError {
    ReadSolutionFile(io::Error),
    LoadProject(project::LoadError),
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

    #[test]
    fn test_extract_project_references() {
        let solution_content = r#"
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

        let project_references = extract_project_references(solution_content);

        assert_eq!(project_references.len(), 2);
        assert_eq!(project_references[0], "Project1/Project1.csproj");
        assert_eq!(project_references[1], "Project2/Project2.csproj");
    }

    #[test]
    fn test_extract_project_references_with_no_projects() {
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
    fn test_extract_project_references_with_solution_folder() {
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

        // Expect only the actual project path
        assert_eq!(project_references.len(), 1);
        assert_eq!(
            project_references[0],
            "SolutionFolder/NestedProject/NestedProject.csproj"
        );
    }

    #[test]
    fn test_extract_project_references_with_solution_items() {
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

        // Expect only the actual project path
        assert_eq!(project_references.len(), 1);
        assert_eq!(
            project_references[0],
            "ProjectWithParams/ProjectWithParams.csproj"
        );
    }
}
