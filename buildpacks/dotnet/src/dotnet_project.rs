use roxmltree::Document;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use thiserror::Error;

use crate::DotnetBuildpackError;

#[derive(Debug)]
pub(crate) struct DotnetProject {
    pub(crate) path: PathBuf,
    #[allow(dead_code)]
    pub(crate) sdk_id: String,
    pub(crate) target_framework: String,
    #[allow(dead_code)]
    pub(crate) project_type: ProjectType,
    #[allow(dead_code)]
    pub(crate) assembly_name: Option<String>,
}

impl DotnetProject {
    pub(crate) fn load_from_path(path: &Path) -> Result<Self, DotnetBuildpackError> {
        parse_project_file_content_from_xml(
            &fs::read_to_string(path).map_err(DotnetBuildpackError::ReadDotnetFile)?,
        )
        .map_err(DotnetBuildpackError::ParseDotnetProjectFile)
        .map(|project_file_contents| Self {
            path: path.to_path_buf(),
            sdk_id: project_file_contents.sdk_id,
            target_framework: project_file_contents.target_framework,
            project_type: project_file_contents.project_type,
            assembly_name: project_file_contents.assembly_name,
        })
    }
}

#[derive(Debug)]
struct ProjectFileContent {
    pub(crate) sdk_id: String,
    pub(crate) target_framework: String,
    pub(crate) project_type: ProjectType,
    pub(crate) assembly_name: Option<String>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ProjectType {
    ConsoleApplication,
    WebApplication,
    RazorApplication,
    BlazorWebAssembly,
    Worker,
    Library,
    Unknown,
}

impl FromStr for ProjectType {
    type Err = ();

    fn from_str(s: &str) -> Result<ProjectType, ()> {
        match s {
            "Microsoft.NET.Sdk" => Ok(ProjectType::Library),
            "Microsoft.NET.Sdk.Web" => Ok(ProjectType::WebApplication),
            "Microsoft.NET.Sdk.Razor" => Ok(ProjectType::RazorApplication),
            "Microsoft.NET.Sdk.BlazorWebAssembly" => Ok(ProjectType::BlazorWebAssembly),
            "Microsoft.NET.Sdk.Worker" => Ok(ProjectType::Worker),
            _ => Ok(ProjectType::Unknown),
        }
    }
}

#[derive(Error, Debug)]
pub(crate) enum ParseError {
    #[error("Error parsing XML")]
    XmlParseError(#[from] roxmltree::Error),
    #[error("No SDK specified")]
    MissingSdkError,
    #[error("Missing TargetFramework")]
    MissingTargetFrameworkError,
}

fn parse_project_file_content_from_xml(
    xml_content: &str,
) -> Result<ProjectFileContent, ParseError> {
    let doc = Document::parse(xml_content)?;

    let mut sdk_id = String::new();
    let mut target_framework = String::new();
    let mut project_type = ProjectType::Unknown;
    let mut assembly_name = None;

    for node in doc.descendants() {
        match node.tag_name().name() {
            "Project" => {
                if let Some(sdk) = node.attribute("Sdk") {
                    sdk_id = sdk.to_string();
                    project_type = sdk_id.parse().unwrap_or(ProjectType::Unknown);
                }
            }
            "Sdk" => {
                if let Some(name) = node.attribute("Name") {
                    sdk_id = name.to_string();
                    project_type = sdk_id.parse().unwrap_or(ProjectType::Unknown);
                } else {
                    sdk_id = node.text().unwrap_or("").to_string();
                    project_type = sdk_id.parse().unwrap_or(ProjectType::Unknown);
                }
            }
            "TargetFramework" => {
                target_framework = node.text().unwrap_or("").to_string();
            }
            "OutputType" => {
                let output_type = node.text().unwrap_or("");
                project_type = match output_type {
                    "Exe" => ProjectType::ConsoleApplication,
                    "Library" => ProjectType::Library,
                    _ => ProjectType::Unknown,
                };
            }
            "AssemblyName" => {
                if let Some(text) = node.text() {
                    if !text.is_empty() {
                        assembly_name = Some(text.to_string());
                    }
                }
            }
            _ => (),
        }
    }

    if sdk_id.is_empty() {
        return Err(ParseError::MissingSdkError);
    }

    if target_framework.is_empty() {
        return Err(ParseError::MissingTargetFrameworkError);
    }

    if sdk_id == "Microsoft.NET.Sdk" && project_type == ProjectType::Unknown {
        project_type = ProjectType::Library;
    }

    Ok(ProjectFileContent {
        sdk_id,
        target_framework,
        project_type,
        assembly_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_console_application_with_sdk_element() {
        let project_xml = r"
<Project>
    <Sdk>Microsoft.NET.Sdk</Sdk>
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
        <OutputType>Exe</OutputType>
    </PropertyGroup>
</Project>
";
        let project = parse_project_file_content_from_xml(project_xml).unwrap();
        assert_eq!(project.sdk_id, "Microsoft.NET.Sdk");
        assert_eq!(project.target_framework, "net6.0");
        assert_eq!(project.project_type, ProjectType::ConsoleApplication);
        assert_eq!(project.assembly_name, None);
    }

    #[test]
    fn test_parse_web_application_with_sdk_attribute() {
        let project_xml = r#"
<Project Sdk="Microsoft.NET.Sdk.Web">
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
    </PropertyGroup>
</Project>
"#;
        let project = parse_project_file_content_from_xml(project_xml).unwrap();
        assert_eq!(project.sdk_id, "Microsoft.NET.Sdk.Web");
        assert_eq!(project.target_framework, "net6.0");
        assert_eq!(project.project_type, ProjectType::WebApplication);
        assert_eq!(project.assembly_name, None);
    }

    #[test]
    fn test_parse_razor_application_with_sdk_element() {
        let project_xml = r#"
<Project>
    <Sdk Name="Microsoft.NET.Sdk.Razor" />
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
    </PropertyGroup>
</Project>
"#;
        let project = parse_project_file_content_from_xml(project_xml).unwrap();
        assert_eq!(project.sdk_id, "Microsoft.NET.Sdk.Razor");
        assert_eq!(project.target_framework, "net6.0");
        assert_eq!(project.project_type, ProjectType::RazorApplication);
        assert_eq!(project.assembly_name, None);
    }

    #[test]
    fn test_parse_blazor_webassembly_application_with_sdk_element() {
        let project_xml = r#"
<Project>
    <Sdk Name="Microsoft.NET.Sdk.BlazorWebAssembly" />
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
    </PropertyGroup>
</Project>
"#;
        let project = parse_project_file_content_from_xml(project_xml).unwrap();
        assert_eq!(project.sdk_id, "Microsoft.NET.Sdk.BlazorWebAssembly");
        assert_eq!(project.target_framework, "net6.0");
        assert_eq!(project.project_type, ProjectType::BlazorWebAssembly);
        assert_eq!(project.assembly_name, None);
    }

    #[test]
    fn test_parse_worker_application_with_sdk_element() {
        let project_xml = r#"
<Project>
    <Sdk Name="Microsoft.NET.Sdk.Worker" />
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
    </PropertyGroup>
</Project>
"#;
        let project = parse_project_file_content_from_xml(project_xml).unwrap();
        assert_eq!(project.sdk_id, "Microsoft.NET.Sdk.Worker");
        assert_eq!(project.target_framework, "net6.0");
        assert_eq!(project.project_type, ProjectType::Worker);
        assert_eq!(project.assembly_name, None);
    }

    #[test]
    fn test_parse_library_project_with_property_group() {
        let project_xml = r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
    </PropertyGroup>
</Project>
"#;
        let project = parse_project_file_content_from_xml(project_xml).unwrap();
        assert_eq!(project.sdk_id, "Microsoft.NET.Sdk");
        assert_eq!(project.target_framework, "net6.0");
        assert_eq!(project.project_type, ProjectType::Library);
        assert_eq!(project.assembly_name, None);
    }

    #[test]
    fn test_parse_project_with_assembly_name() {
        let project_xml = r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
        <AssemblyName>MyAssembly</AssemblyName>
    </PropertyGroup>
</Project>
"#;
        let project = parse_project_file_content_from_xml(project_xml).unwrap();
        assert_eq!(project.sdk_id, "Microsoft.NET.Sdk");
        assert_eq!(project.target_framework, "net6.0");
        assert_eq!(project.project_type, ProjectType::Library);
        assert_eq!(project.assembly_name, Some("MyAssembly".to_string()));
    }

    #[test]
    fn test_parse_project_with_missing_sdk() {
        let project_xml = r"
<Project>
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
        <OutputType>Library</OutputType>
    </PropertyGroup>
</Project>
";
        let result = parse_project_file_content_from_xml(project_xml);
        assert!(matches!(result, Err(ParseError::MissingSdkError)));
    }

    #[test]
    fn test_parse_project_with_missing_target_framework() {
        let project_xml = r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <OutputType>Library</OutputType>
    </PropertyGroup>
</Project>
"#;
        let result = parse_project_file_content_from_xml(project_xml);
        assert!(matches!(
            result,
            Err(ParseError::MissingTargetFrameworkError)
        ));
    }

    #[test]
    fn test_parse_project_with_multiple_property_groups() {
        let project_xml = r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
    </PropertyGroup>
    <PropertyGroup>
        <OutputType>Library</OutputType>
    </PropertyGroup>
</Project>
"#;
        let project = parse_project_file_content_from_xml(project_xml).unwrap();
        assert_eq!(project.sdk_id, "Microsoft.NET.Sdk");
        assert_eq!(project.target_framework, "net6.0");
        assert_eq!(project.project_type, ProjectType::Library);
        assert_eq!(project.assembly_name, None);
    }
}
