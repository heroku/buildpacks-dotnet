use quick_xml::de::from_str;
use serde::Deserialize;
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

        let (target_framework, output_type, assembly_name) = project_xml
            .property_groups
            .iter()
            .fold((None, None, None), |(tf, ot, an), pg| {
                (
                    pg.target_framework.clone().or(tf),
                    pg.output_type.as_deref().or(ot),
                    pg.assembly_name.clone().or(an),
                )
            });

        let target_framework = target_framework
            .ok_or_else(|| LoadError::MissingTargetFramework(path.to_path_buf()))?;

        let sdk_id = project_xml.sdk_id();
        let project_type = infer_project_type(sdk_id, output_type);

        let assembly_name = assembly_name.unwrap_or_else(|| {
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
    #[serde(rename = "Sdk")]
    sdk_element: Option<SdkElement>,
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

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct PropertyGroup {
    target_framework: Option<String>,
    output_type: Option<String>,
    assembly_name: Option<String>,
}

impl ProjectXml {
    fn sdk_id(&self) -> Option<&str> {
        self.sdk.as_deref().or_else(|| {
            self.sdk_element
                .as_ref()
                .and_then(|sdk| sdk.name.as_deref().or(sdk.text.as_deref()))
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

fn infer_project_type(sdk_id: Option<&str>, output_type: Option<&str>) -> ProjectType {
    match sdk_id {
        Some("Microsoft.NET.Sdk") => match output_type {
            Some("Exe") => ProjectType::ConsoleApplication,
            _ => ProjectType::Unknown,
        },
        Some("Microsoft.NET.Sdk.Web" | "Microsoft.NET.Sdk.Razor") => ProjectType::WebApplication,
        Some("Microsoft.NET.Sdk.Worker") => ProjectType::WorkerService,
        _ => ProjectType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn assert_project_type(sdk: &str, output_type: Option<&str>, expected: ProjectType) {
        assert_eq!(infer_project_type(Some(sdk), output_type), expected);
    }

    #[test]
    fn test_parse_default_project_no_sdk() {
        let project_xml = r"
<Project>
</Project>
";
        let project_xml: ProjectXml = from_str(project_xml).unwrap();
        assert_eq!(project_xml.sdk_id(), None);
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
        assert_eq!(project_xml.sdk_id(), Some("Microsoft.NET.Sdk"));
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
        assert_eq!(project_xml.sdk_id(), Some("Microsoft.NET.Sdk.Web"));
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
        assert_eq!(project_xml.sdk_id(), Some("Microsoft.NET.Sdk.Razor"));
    }

    #[test]
    fn test_multiple_property_groups_last_wins() {
        let project_xml = r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>net5.0</TargetFramework>
        <OutputType>Library</OutputType>
        <AssemblyName>FirstName</AssemblyName>
    </PropertyGroup>
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
        <AssemblyName>LastName</AssemblyName>
    </PropertyGroup>
</Project>
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = temp_dir.path().join("test.csproj");
        fs::write(&project_path, project_xml).unwrap();

        let project = Project::load_from_path(&project_path).unwrap();
        assert_eq!(project.target_framework, "net6.0"); // Last value wins
        assert_eq!(project.assembly_name, "LastName"); // Last value wins
        assert_eq!(project.project_type, ProjectType::Unknown); // Library + net sdk = Unknown
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
        let exe_output = Some("Exe");
        assert_eq!(infer_project_type(no_sdk, exe_output), ProjectType::Unknown);
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
