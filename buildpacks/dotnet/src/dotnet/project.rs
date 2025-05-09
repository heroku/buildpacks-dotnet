use roxmltree::Document;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct Project {
    pub(crate) path: PathBuf,
    pub(crate) target_framework: String,
    #[allow(clippy::struct_field_names)]
    pub(crate) project_type: ProjectType,
    pub(crate) assembly_name: String,
}

impl Project {
    pub(crate) fn load_from_path(path: &Path) -> Result<Self, LoadError> {
        let content = fs_err::read_to_string(path).map_err(LoadError::ReadProjectFile)?;
        let metadata =
            parse_metadata(&Document::parse(&content).map_err(LoadError::XmlParseError)?);

        if metadata.target_framework.is_none() {
            return Err(LoadError::MissingTargetFramework(path.to_path_buf()));
        }

        let project_type = infer_project_type(&metadata);

        Ok(Self {
            path: path.to_path_buf(),
            target_framework: metadata
                .target_framework
                .expect("target_framework to be some after earlier check"),
            project_type,
            assembly_name: metadata
                .assembly_name
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| {
                    path.file_stem()
                        .expect("path to have a file name")
                        .to_string_lossy()
                        .to_string()
                }),
        })
    }
}

#[derive(Debug)]
struct Metadata {
    target_framework: Option<String>,
    sdk_id: Option<String>,
    output_type: Option<String>,
    assembly_name: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum ProjectType {
    ConsoleApplication,
    WebApplication,
    WorkerService,
    Unknown,
}

#[derive(Debug)]
pub(crate) enum LoadError {
    ReadProjectFile(io::Error),
    XmlParseError(roxmltree::Error),
    MissingTargetFramework(PathBuf),
}

fn parse_metadata(document: &Document) -> Metadata {
    let mut metadata = Metadata {
        sdk_id: None,
        target_framework: None,
        output_type: None,
        assembly_name: None,
    };

    for node in document.descendants() {
        match node.tag_name().name() {
            "Project" => {
                if let Some(sdk) = node.attribute("Sdk") {
                    metadata.sdk_id = Some(sdk.to_string());
                }
            }
            "Sdk" => {
                if let Some(name) = node.attribute("Name") {
                    metadata.sdk_id = Some(name.to_string());
                } else {
                    metadata.sdk_id = node.text().map(ToString::to_string);
                }
            }
            "TargetFramework" => {
                metadata.target_framework = node.text().map(ToString::to_string);
            }
            "OutputType" => {
                metadata.output_type = node.text().map(ToString::to_string);
            }
            "AssemblyName" => {
                if let Some(text) = node.text() {
                    if !text.trim().is_empty() {
                        metadata.assembly_name = Some(text.to_string());
                    }
                }
            }
            _ => (),
        }
    }
    metadata
}

fn infer_project_type(metadata: &Metadata) -> ProjectType {
    if let Some(sdk_id) = &metadata.sdk_id {
        return match sdk_id.as_str() {
            "Microsoft.NET.Sdk" => match metadata.output_type.as_deref() {
                Some("Exe") => ProjectType::ConsoleApplication,
                _ => ProjectType::Unknown,
            },
            "Microsoft.NET.Sdk.Web" | "Microsoft.NET.Sdk.Razor" => ProjectType::WebApplication,
            "Microsoft.NET.Sdk.Worker" => ProjectType::WorkerService,
            _ => ProjectType::Unknown,
        };
    }
    ProjectType::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn assert_metadata(
        project_xml: &str,
        expected_sdk_id: Option<&str>,
        expected_target_framework: Option<&str>,
        expected_output_type: Option<&str>,
        expected_assembly_name: Option<&str>,
    ) {
        let metadata = parse_metadata(&Document::parse(project_xml).unwrap());
        assert_eq!(metadata.sdk_id, expected_sdk_id.map(ToString::to_string));
        assert_eq!(
            metadata.target_framework,
            expected_target_framework.map(String::from)
        );
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
        assert_metadata(
            project_xml,
            Some("Microsoft.NET.Sdk"),
            Some("net6.0"),
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
        assert_metadata(
            project_xml,
            Some("Microsoft.NET.Sdk.Web"),
            Some("net6.0"),
            None,
            None,
        );
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
        assert_metadata(
            project_xml,
            Some("Microsoft.NET.Sdk.Razor"),
            Some("net6.0"),
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
        assert_metadata(
            project_xml,
            Some("Microsoft.NET.Sdk.BlazorWebAssembly"),
            Some("net6.0"),
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
        assert_metadata(
            project_xml,
            Some("Microsoft.NET.Sdk.Worker"),
            Some("net6.0"),
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
        assert_metadata(
            project_xml,
            Some("Microsoft.NET.Sdk"),
            Some("net6.0"),
            None,
            None,
        );
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
        assert_metadata(
            project_xml,
            Some("Microsoft.NET.Sdk"),
            Some("net6.0"),
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
        assert_metadata(project_xml, None, Some("net6.0"), Some("Library"), None);
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
        assert_metadata(
            project_xml,
            Some("Microsoft.NET.Sdk"),
            Some("net6.0"),
            Some("Library"),
            None,
        );
    }

    #[test]
    fn test_infer_project_type_console_application() {
        let metadata = Metadata {
            sdk_id: Some("Microsoft.NET.Sdk".to_string()),
            target_framework: Some("net6.0".to_string()),
            output_type: Some("Exe".to_string()),
            assembly_name: None,
        };
        assert_eq!(
            infer_project_type(&metadata),
            ProjectType::ConsoleApplication
        );
    }

    #[test]
    fn test_infer_project_type_worker() {
        let metadata = Metadata {
            sdk_id: Some("Microsoft.NET.Sdk.Worker".to_string()),
            target_framework: Some("net6.0".to_string()),
            output_type: None,
            assembly_name: None,
        };
        assert_eq!(infer_project_type(&metadata), ProjectType::WorkerService);
    }

    #[test]
    fn test_infer_project_type_web_application() {
        let metadata = Metadata {
            sdk_id: Some("Microsoft.NET.Sdk.Web".to_string()),
            target_framework: Some("net6.0".to_string()),
            output_type: None,
            assembly_name: None,
        };
        assert_eq!(infer_project_type(&metadata), ProjectType::WebApplication);

        let metadata = Metadata {
            sdk_id: Some("Microsoft.NET.Sdk.Razor".to_string()),
            target_framework: Some("net6.0".to_string()),
            output_type: None,
            assembly_name: None,
        };
        assert_eq!(infer_project_type(&metadata), ProjectType::WebApplication);
    }

    #[test]
    fn test_infer_project_type_unknown() {
        let metadata = Metadata {
            sdk_id: Some("Unknown.Sdk".to_string()),
            target_framework: Some("net6.0".to_string()),
            output_type: None,
            assembly_name: None,
        };
        assert_eq!(infer_project_type(&metadata), ProjectType::Unknown);
    }

    #[test]
    fn test_parse_project_with_empty_target_framework() {
        let project_xml = r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework></TargetFramework>
    </PropertyGroup>
</Project>
"#;
        assert_metadata(project_xml, Some("Microsoft.NET.Sdk"), None, None, None);
    }

    #[test]
    fn test_parse_project_with_empty_assembly_name() {
        let project_xml = r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
        <AssemblyName></AssemblyName>
    </PropertyGroup>
</Project>
"#;
        assert_metadata(
            project_xml,
            Some("Microsoft.NET.Sdk"),
            Some("net6.0"),
            None,
            None,
        );
    }

    #[test]
    fn test_parse_project_with_whitespace_assembly_name() {
        let project_xml = r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
        <AssemblyName>  </AssemblyName>
    </PropertyGroup>
</Project>
"#;
        assert_metadata(
            project_xml,
            Some("Microsoft.NET.Sdk"),
            Some("net6.0"),
            None,
            None,
        );
    }

    #[test]
    fn test_infer_project_type_unknown_sdk_with_exe() {
        let metadata = Metadata {
            sdk_id: Some("Unknown.Sdk".to_string()),
            target_framework: Some("net6.0".to_string()),
            output_type: Some("Exe".to_string()),
            assembly_name: None,
        };
        assert_eq!(infer_project_type(&metadata), ProjectType::Unknown);
    }

    #[test]
    fn test_infer_project_type_net_sdk_without_exe() {
        let metadata = Metadata {
            sdk_id: Some("Microsoft.NET.Sdk".to_string()),
            target_framework: Some("net6.0".to_string()),
            output_type: Some("Library".to_string()),
            assembly_name: None,
        };
        assert_eq!(infer_project_type(&metadata), ProjectType::Unknown);
    }

    #[test]
    fn test_infer_project_type_no_sdk() {
        let metadata = Metadata {
            sdk_id: None,
            target_framework: Some("net6.0".to_string()),
            output_type: Some("Exe".to_string()),
            assembly_name: None,
        };
        assert_eq!(infer_project_type(&metadata), ProjectType::Unknown);
    }

    #[test]
    fn test_load_project_missing_target_framework() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = temp_dir.path().join("test.csproj");
        fs::write(
            &project_path,
            r#"
<Project Sdk="Microsoft.NET.Sdk">
</Project>"#,
        )
        .unwrap();

        let result = Project::load_from_path(&project_path);
        assert!(matches!(result, Err(LoadError::MissingTargetFramework(_))));
    }

    #[test]
    fn test_load_project_with_assembly_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = temp_dir.path().join("ConsoleApp.csproj");
        fs::write(
            &project_path,
            r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
        <OutputType>Exe</OutputType>
        <AssemblyName>MyConsoleApp</AssemblyName>
    </PropertyGroup>
</Project>"#,
        )
        .unwrap();

        let project = Project::load_from_path(&project_path).unwrap();
        assert_eq!(project.target_framework, "net6.0".to_string());
        assert_eq!(project.project_type, ProjectType::ConsoleApplication);
        assert_eq!(project.assembly_name, "MyConsoleApp");
        assert_eq!(project.path, project_path);
    }

    #[test]
    fn test_load_project_without_assembly_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = temp_dir.path().join("ConsoleApp.csproj");
        fs::write(
            &project_path,
            r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
        <OutputType>Exe</OutputType>
    </PropertyGroup>
</Project>"#,
        )
        .unwrap();

        let project = Project::load_from_path(&project_path).unwrap();
        assert_eq!(project.target_framework, "net6.0".to_string());
        assert_eq!(project.project_type, ProjectType::ConsoleApplication);
        assert_eq!(
            project.assembly_name,
            project_path.file_stem().unwrap().to_string_lossy()
        );
        assert_eq!(project.path, project_path);
    }
}
