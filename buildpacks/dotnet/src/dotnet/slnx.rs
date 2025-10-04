use serde::Deserialize;

#[derive(Deserialize)]
struct Solution {
    #[serde(rename = "Project", default)]
    projects: Vec<Project>,
    #[serde(rename = "Folder", default)]
    folders: Vec<Folder>,
}

#[derive(Deserialize)]
struct Folder {
    #[serde(rename = "Project", default)]
    projects: Vec<Project>,
}

#[derive(Deserialize)]
struct Project {
    #[serde(rename = "@Path")]
    path: String,
}

pub(crate) fn extract_project_paths(xml_content: &str) -> Result<Vec<String>, quick_xml::DeError> {
    let solution: Solution = quick_xml::de::from_str(xml_content)?;
    Ok(solution
        .projects
        .iter()
        .chain(solution.folders.iter().flat_map(|folder| &folder.projects))
        .map(|project| project.path.replace('\\', "/"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_projects_from_mixed_structure() {
        let slnx_content = r#"
<Solution>
  <Project Path="RootProject\RootProject.csproj" />
  <Folder Name="/Application/">
    <Project Path="FolderProject\FolderProject.csproj" />
  </Folder>
</Solution>
"#;
        let projects = extract_project_paths(slnx_content).unwrap();

        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0], "RootProject/RootProject.csproj");
        assert_eq!(projects[1], "FolderProject/FolderProject.csproj");
    }

    #[test]
    fn test_ignore_non_project_elements() {
        let slnx_content = r#"
<Solution>
  <Configurations>
    <Platform Name="Any CPU" />
    <Platform Name="x64" />
    <Platform Name="x86" />
  </Configurations>
  <Folder Name="/EmptyFolder/" />
  <Folder Name="/Solution Items/">
    <File Path="Directory.Build.props" />
  </Folder>
  <Project Path="App/App.csproj" />
</Solution>
"#;
        let projects = extract_project_paths(slnx_content).unwrap();
        assert_eq!(projects, vec!["App/App.csproj"]);
    }

    #[test]
    fn test_malformed_xml() {
        let slnx_content = r#"
<Solution>
  <Project Path="App/App.csproj" />
"#;
        let result = extract_project_paths(slnx_content);
        assert!(result.is_err());
    }
}
