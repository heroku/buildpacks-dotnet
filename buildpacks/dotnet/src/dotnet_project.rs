use roxmltree::Document;
use std::path::{Path, PathBuf};
use std::{fs, io};
use thiserror::Error;

#[derive(Debug)]
pub(crate) struct DotnetProject {
    pub(crate) path: PathBuf,
    pub(crate) target_framework: String,
    #[allow(dead_code)]
    pub(crate) project_type: ProjectType,
    #[allow(dead_code)]
    pub(crate) assembly_name: Option<String>,
}

impl DotnetProject {
    pub(crate) fn load_from_path(path: &Path) -> Result<Self, LoadProjectError> {
        let content = fs::read_to_string(path).map_err(LoadProjectError::ReadProjectFile)?;
        let metadata = parse_dotnet_project_metadata(
            &Document::parse(&content).map_err(LoadProjectError::XmlParseError)?,
        );

        if metadata.target_framework.is_empty() {
            return Err(LoadProjectError::MissingTargetFramework);
        }

        let project_type = infer_project_type(&metadata);

        Ok(Self {
            path: path.to_path_buf(),
            target_framework: metadata.target_framework,
            project_type,
            assembly_name: metadata.assembly_name,
        })
    }
}

#[derive(Debug)]
struct DotnetProjectMetadata {
    sdk_id: String,
    target_framework: String,
    output_type: Option<String>,
    assembly_name: Option<String>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ProjectType {
    ConsoleApplication,
    WebApplication,
    Unknown,
}

#[derive(Error, Debug)]
pub(crate) enum LoadProjectError {
    #[error("Error reading project file")]
    ReadProjectFile(io::Error),
    #[error("Error parsing XML")]
    XmlParseError(#[from] roxmltree::Error),
    #[error("Missing TargetFramework")]
    MissingTargetFramework,
}

fn parse_dotnet_project_metadata(document: &Document) -> DotnetProjectMetadata {
    let mut metadata = DotnetProjectMetadata {
        sdk_id: String::new(),
        target_framework: String::new(),
        output_type: None,
        assembly_name: None,
    };

    for node in document.descendants() {
        match node.tag_name().name() {
            "Project" => {
                if let Some(sdk) = node.attribute("Sdk") {
                    metadata.sdk_id = sdk.to_string();
                }
            }
            "Sdk" => {
                if let Some(name) = node.attribute("Name") {
                    metadata.sdk_id = name.to_string();
                } else {
                    metadata.sdk_id = node.text().unwrap_or("").to_string();
                }
            }
            "TargetFramework" => {
                metadata.target_framework = node.text().unwrap_or("").to_string();
            }
            "OutputType" => {
                metadata.output_type = node.text().map(ToString::to_string);
            }
            "AssemblyName" => {
                if let Some(text) = node.text() {
                    if !text.is_empty() {
                        metadata.assembly_name = Some(text.to_string());
                    }
                }
            }
            _ => (),
        }
    }
    metadata
}

fn infer_project_type(metadata: &DotnetProjectMetadata) -> ProjectType {
    match metadata.sdk_id.as_str() {
        "Microsoft.NET.Sdk" => match metadata.output_type.as_deref() {
            Some("Exe") => ProjectType::ConsoleApplication,
            _ => ProjectType::Unknown,
        },
        "Microsoft.NET.Sdk.Web" | "Microsoft.NET.Sdk.Razor" => ProjectType::WebApplication,
        _ => ProjectType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_dotnet_project_metadata(
        project_xml: &str,
        expected_sdk_id: &str,
        expected_target_framework: &str,
        expected_output_type: Option<&str>,
        expected_assembly_name: Option<&str>,
    ) {
        let metadata = parse_dotnet_project_metadata(&Document::parse(project_xml).unwrap());
        assert_eq!(metadata.sdk_id, expected_sdk_id);
        assert_eq!(metadata.target_framework, expected_target_framework);
        assert_eq!(metadata.output_type, expected_output_type.map(String::from));
        assert_eq!(
            metadata.assembly_name,
            expected_assembly_name.map(String::from)
        );
    }

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
        assert_dotnet_project_metadata(
            project_xml,
            "Microsoft.NET.Sdk",
            "net6.0",
            Some("Exe"),
            None,
        );
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
        assert_dotnet_project_metadata(project_xml, "Microsoft.NET.Sdk.Web", "net6.0", None, None);
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
        assert_dotnet_project_metadata(
            project_xml,
            "Microsoft.NET.Sdk.Razor",
            "net6.0",
            None,
            None,
        );
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
        assert_dotnet_project_metadata(
            project_xml,
            "Microsoft.NET.Sdk.BlazorWebAssembly",
            "net6.0",
            None,
            None,
        );
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
        assert_dotnet_project_metadata(
            project_xml,
            "Microsoft.NET.Sdk.Worker",
            "net6.0",
            None,
            None,
        );
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
        assert_dotnet_project_metadata(project_xml, "Microsoft.NET.Sdk", "net6.0", None, None);
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
        assert_dotnet_project_metadata(
            project_xml,
            "Microsoft.NET.Sdk",
            "net6.0",
            None,
            Some("MyAssembly"),
        );
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
        assert_dotnet_project_metadata(project_xml, "", "net6.0", Some("Library"), None);
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
        assert_dotnet_project_metadata(
            project_xml,
            "Microsoft.NET.Sdk",
            "net6.0",
            Some("Library"),
            None,
        );
    }

    #[test]
    fn test_infer_project_type_console_application() {
        let metadata = DotnetProjectMetadata {
            sdk_id: "Microsoft.NET.Sdk".to_string(),
            target_framework: "net6.0".to_string(),
            output_type: Some("Exe".to_string()),
            assembly_name: None,
        };
        assert_eq!(
            infer_project_type(&metadata),
            ProjectType::ConsoleApplication
        );
    }

    #[test]
    fn test_infer_project_type_web_application() {
        let metadata = DotnetProjectMetadata {
            sdk_id: "Microsoft.NET.Sdk.Web".to_string(),
            target_framework: "net6.0".to_string(),
            output_type: None,
            assembly_name: None,
        };
        assert_eq!(infer_project_type(&metadata), ProjectType::WebApplication);

        let metadata = DotnetProjectMetadata {
            sdk_id: "Microsoft.NET.Sdk.Razor".to_string(),
            target_framework: "net6.0".to_string(),
            output_type: None,
            assembly_name: None,
        };
        assert_eq!(infer_project_type(&metadata), ProjectType::WebApplication);
    }

    #[test]
    fn test_infer_project_type_unknown() {
        let metadata = DotnetProjectMetadata {
            sdk_id: "Unknown.Sdk".to_string(),
            target_framework: "net6.0".to_string(),
            output_type: None,
            assembly_name: None,
        };
        assert_eq!(infer_project_type(&metadata), ProjectType::Unknown);
    }
}
