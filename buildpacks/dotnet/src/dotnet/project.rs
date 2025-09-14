use quick_xml::de::from_str;
use serde::{Deserialize, Deserializer};
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
        let project_xml: ProjectXml = from_str(&content).map_err(LoadError::XmlParseError)?;

        let target_framework = project_xml
            .property_groups
            .iter()
            .find_map(|pg| pg.target_framework.clone())
            .ok_or_else(|| LoadError::MissingTargetFramework(path.to_path_buf()))?;

        let sdk_id = project_xml.sdk_id();
        let output_type = project_xml
            .property_groups
            .iter()
            .find_map(|pg| pg.output_type.clone());
        let project_type = infer_project_type(sdk_id.as_ref(), output_type.as_ref());

        let assembly_name = project_xml
            .property_groups
            .iter()
            .find_map(|pg| pg.assembly_name.clone())
            .unwrap_or_else(|| {
                path.file_stem()
                    .expect("path to have a file name")
                    .to_string_lossy()
                    .to_string()
            });

        Ok(Self {
            path: path.to_path_buf(),
            target_framework,
            project_type,
            assembly_name,
        })
    }
}

#[derive(Debug, Deserialize)]
struct ProjectXml {
    #[serde(rename = "@Sdk")]
    sdk: Option<String>,
    #[serde(rename = "Sdk", default)]
    sdk_elements: Vec<SdkElement>,
    #[serde(rename = "PropertyGroup", default)]
    property_groups: Vec<PropertyGroup>,
}

#[derive(Debug, Deserialize)]
struct SdkElement {
    #[serde(rename = "@Name")]
    name: Option<String>,
    #[serde(rename = "$text")]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PropertyGroup {
    #[serde(
        rename = "TargetFramework",
        default,
        deserialize_with = "deserialize_non_empty_string"
    )]
    target_framework: Option<String>,
    #[serde(
        rename = "OutputType",
        default,
        deserialize_with = "deserialize_non_empty_string"
    )]
    output_type: Option<String>,
    #[serde(
        rename = "AssemblyName",
        default,
        deserialize_with = "deserialize_non_empty_string"
    )]
    assembly_name: Option<String>,
}

impl ProjectXml {
    fn sdk_id(&self) -> Option<String> {
        self.sdk.clone().or_else(|| {
            self.sdk_elements
                .iter()
                .find_map(|sdk| sdk.name.clone().or_else(|| sdk.text.clone()))
        })
    }
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
    XmlParseError(quick_xml::de::DeError),
    MissingTargetFramework(PathBuf),
}

fn infer_project_type(sdk_id: Option<&String>, output_type: Option<&String>) -> ProjectType {
    match sdk_id.map(String::as_str) {
        Some("Microsoft.NET.Sdk") => match output_type.map(String::as_str) {
            Some("Exe") => ProjectType::ConsoleApplication,
            _ => ProjectType::Unknown,
        },
        Some("Microsoft.NET.Sdk.Web" | "Microsoft.NET.Sdk.Razor") => ProjectType::WebApplication,
        Some("Microsoft.NET.Sdk.Worker") => ProjectType::WorkerService,
        _ => ProjectType::Unknown,
    }
}

fn deserialize_non_empty_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.filter(|s| !s.trim().is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // Helper to test project type inference without XML parsing
    fn assert_project_type(sdk: &str, output_type: Option<&str>, expected: ProjectType) {
        let sdk_id = Some(sdk.to_string());
        let output_type = output_type.map(String::from);
        assert_eq!(
            infer_project_type(sdk_id.as_ref(), output_type.as_ref()),
            expected
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
        let project_xml: ProjectXml = from_str(project_xml).unwrap();
        assert_eq!(project_xml.sdk_id(), Some("Microsoft.NET.Sdk".to_string()));
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
        let project_xml: ProjectXml = from_str(project_xml).unwrap();
        assert_eq!(
            project_xml.sdk_id(),
            Some("Microsoft.NET.Sdk.Web".to_string())
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
        let project_xml: ProjectXml = from_str(project_xml).unwrap();
        assert_eq!(
            project_xml.sdk_id(),
            Some("Microsoft.NET.Sdk.Razor".to_string())
        );
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
        let project_xml: ProjectXml = from_str(project_xml).unwrap();
        assert_eq!(
            project_xml
                .property_groups
                .iter()
                .find_map(|pg| pg.target_framework.clone()),
            Some("net6.0".to_string())
        );
        assert_eq!(
            project_xml
                .property_groups
                .iter()
                .find_map(|pg| pg.output_type.clone()),
            Some("Library".to_string())
        );
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
        let project_xml: ProjectXml = from_str(project_xml).unwrap();
        assert_eq!(
            project_xml
                .property_groups
                .iter()
                .find_map(|pg| pg.target_framework.clone()),
            None
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
        let project_xml: ProjectXml = from_str(project_xml).unwrap();
        assert_eq!(
            project_xml
                .property_groups
                .iter()
                .find_map(|pg| pg.assembly_name.clone()),
            None
        );
    }

    #[test]
    fn test_console_application_inference() {
        assert_project_type(
            "Microsoft.NET.Sdk",
            Some("Exe"),
            ProjectType::ConsoleApplication,
        );
    }

    #[test]
    fn test_web_application_inference() {
        assert_project_type("Microsoft.NET.Sdk.Web", None, ProjectType::WebApplication);
        assert_project_type("Microsoft.NET.Sdk.Razor", None, ProjectType::WebApplication);
    }

    #[test]
    fn test_worker_service_inference() {
        assert_project_type("Microsoft.NET.Sdk.Worker", None, ProjectType::WorkerService);
    }

    #[test]
    fn test_unknown_project_types() {
        assert_project_type("Unknown.Sdk", None, ProjectType::Unknown);
        assert_project_type("Unknown.Sdk", Some("Exe"), ProjectType::Unknown);
        assert_project_type("Microsoft.NET.Sdk", Some("Library"), ProjectType::Unknown);

        // No SDK case
        let no_sdk = None;
        let exe_output = Some("Exe".to_string());
        assert_eq!(
            infer_project_type(no_sdk, exe_output.as_ref()),
            ProjectType::Unknown
        );
    }

    #[test]
    fn test_missing_target_framework_error() {
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
