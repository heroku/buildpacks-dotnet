use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

/// Parses a .NET solution file and extracts a list of project file paths.
///
/// # Arguments
///
/// * `path` - A path to the .NET solution file.
///
/// # Returns
///
/// * `Ok(Vec<String>)` - A vector of relative project file paths if parsing is successful.
/// * `Err(io::Error)` - An I/O error if reading the file fails.
pub(crate) fn parse_solution_file<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);
    extract_project_paths(reader)
}

/// Extracts project file paths from a .NET solution file.
///
/// # Arguments
///
/// * `reader` - A buffered reader for the solution file.
///
/// # Returns
///
/// * `Ok(Vec<String>)` - A vector of project file paths if parsing is successful.
/// * `Err(io::Error)` - An I/O error if reading the file fails.
fn extract_project_paths<R: BufRead>(reader: R) -> io::Result<Vec<String>> {
    let project_line_regex =
        Regex::new(r#"Project\("\{[^}]+\}"\) = "[^"]+", "([^"]+\.[^"]+)", "\{[^}]+\}""#)
            .expect("regex to be valid");
    let mut project_paths = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if let Some(captures) = project_line_regex.captures(&line) {
            if let Some(project_path) = captures.get(1) {
                // Normalize the path to use forward slashes
                let normalized_path = project_path.as_str().replace('\\', "/");
                project_paths.push(normalized_path);
            }
        }
    }

    Ok(project_paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_solution_file() {
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

        let cursor = Cursor::new(solution_content);

        let project_paths = extract_project_paths(cursor).unwrap();

        assert_eq!(project_paths.len(), 2);
        assert_eq!(project_paths[0], "Project1/Project1.csproj");
        assert_eq!(project_paths[1], "Project2/Project2.csproj");
    }

    #[test]
    fn test_parse_solution_file_with_no_projects() {
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

        let cursor = Cursor::new(solution_content);

        let project_paths = extract_project_paths(cursor).unwrap();

        assert_eq!(project_paths.len(), 0);
    }

    #[test]
    fn test_parse_solution_file_with_solution_folder() {
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

        let cursor = Cursor::new(solution_content);

        let project_paths = extract_project_paths(cursor).unwrap();

        // Expect only the actual project path
        assert_eq!(project_paths.len(), 1);
        assert_eq!(
            project_paths[0],
            "SolutionFolder/NestedProject/NestedProject.csproj"
        );
    }

    #[test]
    fn test_parse_solution_file_with_solution_items() {
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

        let cursor = Cursor::new(solution_content);

        let project_paths = extract_project_paths(cursor).unwrap();

        // Expect only the actual project path
        assert_eq!(project_paths.len(), 1);
        assert_eq!(
            project_paths[0],
            "ProjectWithParams/ProjectWithParams.csproj"
        );
    }
}
