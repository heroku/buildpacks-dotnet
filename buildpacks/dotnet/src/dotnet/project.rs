use crate::detect::directory_build_props_file;
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

        let property_groups = &project_xml.property_groups;

        // Try to find TargetFramework in project file first, then Directory.Build.props
        let target_framework = match extract_target_framework(property_groups) {
            Some(tfm) => tfm,
            None => find_target_framework_from_directory_build_props(path)?
                .ok_or_else(|| LoadError::MissingTargetFramework(path.to_path_buf()))?,
        };

        // Find the last one, but if it's blank, fall back to the file name
        // (even if an earlier, non-empty/whitespace assembly name is set).
        // This is consistent with MSBuild's own behavior
        let assembly_name = property_groups
            .iter()
            .filter_map(|pg| pg.assembly_name.as_ref())
            .next_back()
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| {
                path.file_stem()
                    .expect("A path that can be read must have a file stem")
                    .to_string_lossy()
                    .to_string()
            });

        let output_type = property_groups
            .iter()
            .filter_map(|pg| pg.output_type.as_deref())
            .next_back();

        let project_type = project_xml
            .sdk_element
            .map(|sdk_element| sdk_element.name)
            .or(project_xml.sdk)
            .map_or(ProjectType::Unknown, |sdk_id| {
                infer_project_type(&sdk_id, output_type)
            });

        Ok(Self {
            path: path.to_path_buf(),
            target_framework,
            project_type,
            assembly_name,
        })
    }

    pub(crate) fn load_from_file_based_app(path: &Path) -> Result<Self, io::Error> {
        let content = fs_err::read_to_string(path)?;

        let mut sdk_id: Option<&str> = None;
        let mut target_framework: Option<&str> = None;

        for line in content.lines() {
            let trimmed_line = line.trim();

            // Find the first SDK then stop looking for it (only the first sdk directive maps to the project SDK)
            if sdk_id.is_none()
                && let Some(sdk_val) = trimmed_line.strip_prefix("#:sdk ")
            {
                sdk_id = Some(sdk_val);
            }

            // Find the *first* TargetFramework. Specifying duplicate properties will cause an error during
            // publish, but for now we let the `dotnet` CLI provide that error feedback later.
            if target_framework.is_none()
                && let Some(tfm_val) = trimmed_line.strip_prefix("#:property TargetFramework=")
            {
                target_framework = Some(tfm_val);
            }

            if sdk_id.is_some() && target_framework.is_some() {
                break;
            }
        }

        // Apply defaults if values were not found in the file
        let final_sdk_id = sdk_id.unwrap_or("Microsoft.NET.Sdk");
        let final_target_framework = if let Some(tfm) = target_framework {
            tfm.to_string()
        } else {
            find_target_framework_from_directory_build_props(path)
                .unwrap() // Handle error more gracefully
                .unwrap_or_else(|| "net10.0".to_string())
        };
        // File-based apps are executables, so pass 'Exe' as the output type when
        // when inferring project type (e.g. default to ConsoleApplication).
        let project_type = infer_project_type(final_sdk_id, Some("Exe"));

        // File-based apps use the file stem as the assembly name, just like project file defaults.
        // Unlike project files, setting the AssemblyName property doesn't change the output.
        let assembly_name = path
            .file_stem()
            .expect("A path that can be read must have a file stem")
            .to_string_lossy()
            .to_string();

        Ok(Self {
            path: path.to_path_buf(),
            target_framework: final_target_framework,
            project_type,
            assembly_name,
        })
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct PropertyGroup {
    target_framework: Option<String>,
    output_type: Option<String>,
    assembly_name: Option<String>,
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
struct DirectoryBuildPropsXml {
    #[serde(rename = "PropertyGroup", default)]
    property_groups: Vec<PropertyGroup>,
}

#[derive(Debug, Deserialize)]
struct SdkElement {
    #[serde(rename = "@Name")]
    name: String,
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
    ReadDirectoryBuildProps(io::Error),
    XmlParseDirectoryBuildProps(quick_xml::de::DeError),
}

fn infer_project_type(sdk_id: &str, output_type: Option<&str>) -> ProjectType {
    match sdk_id {
        "Microsoft.NET.Sdk" => match output_type {
            Some("Exe") => ProjectType::ConsoleApplication,
            _ => ProjectType::Unknown,
        },
        "Microsoft.NET.Sdk.Web" | "Microsoft.NET.Sdk.Razor" => ProjectType::WebApplication,
        "Microsoft.NET.Sdk.Worker" => ProjectType::WorkerService,
        _ => ProjectType::Unknown,
    }
}


fn extract_target_framework(property_groups: &[PropertyGroup]) -> Option<String> {
    // Find the last TargetFramework property (last wins)
    property_groups
        .iter()
        .filter_map(|pg| pg.target_framework.as_ref())
        .next_back()
        .cloned()
}

fn find_target_framework_from_directory_build_props(
    file_path: &Path,
) -> Result<Option<String>, LoadError> {
    let Some(props_path) = file_path.parent().and_then(directory_build_props_file) else {
        return Ok(None);
    };

    let content =
        fs_err::read_to_string(&props_path).map_err(LoadError::ReadDirectoryBuildProps)?;
    let props_xml: DirectoryBuildPropsXml =
        from_str(&content).map_err(LoadError::XmlParseDirectoryBuildProps)?;

    Ok(extract_target_framework(&props_xml.property_groups))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::ErrorKind;

    #[test]
    fn test_sdk_attribute_resolution() {
        let project_xml = r#"
<Project Sdk="Microsoft.NET.Sdk.Web">
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
    </PropertyGroup>
</Project>
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = temp_dir.path().join("WebApp.csproj");
        fs::write(&project_path, project_xml).unwrap();

        let project = Project::load_from_path(&project_path).unwrap();
        assert_eq!(project.project_type, ProjectType::WebApplication);
    }

    #[test]
    fn test_sdk_element_resolution() {
        let project_xml = r#"
<Project>
    <Sdk Name="Microsoft.NET.Sdk.Razor" />
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
    </PropertyGroup>
</Project>
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = temp_dir.path().join("RazorApp.csproj");
        fs::write(&project_path, project_xml).unwrap();

        let project = Project::load_from_path(&project_path).unwrap();
        assert_eq!(project.project_type, ProjectType::WebApplication);
    }

    #[test]
    fn test_no_sdk_resolution() {
        let project_xml = r"
<Project>
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
    </PropertyGroup>
</Project>
";
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = temp_dir.path().join("NoSdk.csproj");
        fs::write(&project_path, project_xml).unwrap();

        let project = Project::load_from_path(&project_path).unwrap();
        assert_eq!(project.project_type, ProjectType::Unknown);
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
        <AssemblyName>  </AssemblyName>
    </PropertyGroup>
</Project>
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = temp_dir.path().join("test.csproj");
        fs::write(&project_path, project_xml).unwrap();

        let project = Project::load_from_path(&project_path).unwrap();
        assert_eq!(project.target_framework, "net6.0"); // Last value wins
        assert_eq!(project.assembly_name, "test"); // Falls back to filename when whitespace
        assert_eq!(project.project_type, ProjectType::Unknown);
    }

    #[test]
    fn test_project_type_inference() {
        assert_eq!(
            infer_project_type("Microsoft.NET.Sdk", Some("Exe")),
            ProjectType::ConsoleApplication
        );

        assert_eq!(
            infer_project_type("Microsoft.NET.Sdk.Web", None),
            ProjectType::WebApplication
        );
        assert_eq!(
            infer_project_type("Microsoft.NET.Sdk.Razor", None),
            ProjectType::WebApplication
        );

        assert_eq!(
            infer_project_type("Microsoft.NET.Sdk.Worker", None),
            ProjectType::WorkerService
        );

        assert_eq!(
            infer_project_type("Unknown.Sdk", None),
            ProjectType::Unknown
        );
        assert_eq!(
            infer_project_type("Unknown.Sdk", Some("Exe")),
            ProjectType::Unknown
        );
        assert_eq!(
            infer_project_type("Microsoft.NET.Sdk", Some("Library")),
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
        assert_matches!(result, Err(LoadError::MissingTargetFramework(path)) if path == &project_path);
    }

    #[test]
    fn test_read_project_file_error() {
        let nonexistent_path = Path::new("/nonexistent/path/test.csproj");
        let result = Project::load_from_path(nonexistent_path).unwrap_err();

        assert_matches!(result, LoadError::ReadProjectFile(error) if error.kind() == ErrorKind::NotFound);
    }

    #[test]
    fn test_xml_parse_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = temp_dir.path().join("test.csproj");
        fs::write(&project_path, "not valid xml").unwrap();

        let result = Project::load_from_path(&project_path);
        assert_matches!(result, Err(LoadError::XmlParseError(_)));
    }

    #[test]
    fn test_load_file_based_app_io_error() {
        let nonexistent_path = Path::new("/nonexistent/path/test.cs");
        let result = Project::load_from_file_based_app(nonexistent_path);

        assert_matches!(result, Err(error) if error.kind() == ErrorKind::NotFound);
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

    #[test]
    fn test_load_file_based_app_defaults() {
        let project_cs = r#"
Console.WriteLine("foobar");
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let app_path = temp_dir.path().join("DefaultApp.cs");
        fs::write(&app_path, project_cs).unwrap();

        let project = Project::load_from_file_based_app(&app_path).unwrap();

        // Should default to "Microsoft.NET.Sdk" and "Exe" output, so we expect ConsoleApplication
        assert_eq!(project.project_type, ProjectType::ConsoleApplication);
        // Should default to "net10.0"
        assert_eq!(project.target_framework, "net10.0");
        assert_eq!(project.assembly_name, "DefaultApp");
    }

    #[test]
    fn test_load_file_based_app_explicit_configuration() {
        let project_cs = r#"
#:sdk Microsoft.NET.Sdk.Web
#:sdk Aspire.AppHost.Sdk@9.4.1
#:property TargetFramework=net11.0
#:property LangVersion=preview

Console.WriteLine("foobar");
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let app_path = temp_dir.path().join("MyApp.cs");
        fs::write(&app_path, project_cs).unwrap();

        let project = Project::load_from_file_based_app(&app_path).unwrap();

        // It should find the *first* SDK
        assert_eq!(project.project_type, ProjectType::WebApplication);
        // It should find the TargetFramework
        assert_eq!(project.target_framework, "net11.0");
        // Assembly name should be file stem
        assert_eq!(project.assembly_name, "MyApp");
        assert_eq!(project.path, app_path);
    }

    #[test]
    fn test_load_file_based_app_mixed_defaults() {
        let project_cs = r#"
#:sdk Microsoft.NET.Sdk.Worker

Console.WriteLine("foobar");
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let app_path = temp_dir.path().join("WorkerApp.cs");
        fs::write(&app_path, project_cs).unwrap();

        let project = Project::load_from_file_based_app(&app_path).unwrap();

        assert_eq!(project.project_type, ProjectType::WorkerService);
        assert_eq!(project.target_framework, "net10.0");
        assert_eq!(project.assembly_name, "WorkerApp");
    }

    #[test]
    fn test_load_file_based_app_with_directory_build_props() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create Directory.Build.props with TargetFramework
        fs::write(
            temp_dir.path().join("Directory.Build.props"),
            r"
<Project>
    <PropertyGroup>
        <TargetFramework>net9.0</TargetFramework>
    </PropertyGroup>
</Project>",
        )
        .unwrap();

        // Create file-based app without TargetFramework property
        let app_path = temp_dir.path().join("MyApp.cs");
        fs::write(
            &app_path,
            r#"
#:sdk Microsoft.NET.Sdk

Console.WriteLine("Hello from file-based app!");
"#,
        )
        .unwrap();

        let project = Project::load_from_file_based_app(&app_path).unwrap();

        assert_eq!(project.target_framework, "net9.0");
        assert_eq!(project.project_type, ProjectType::ConsoleApplication);
        assert_eq!(project.assembly_name, "MyApp");
    }

    #[test]
    fn test_load_project_with_directory_build_props() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create Directory.Build.props with TargetFramework
        fs::write(
            temp_dir.path().join("Directory.Build.props"),
            r"
<Project>
    <PropertyGroup>
        <TargetFramework>net8.0</TargetFramework>
    </PropertyGroup>
</Project>",
        )
        .unwrap();

        // Create subdirectory with project that doesn't specify TargetFramework
        let project_dir = temp_dir.path().join("MyProject");
        fs::create_dir(&project_dir).unwrap();
        let project_path = project_dir.join("MyProject.csproj");
        fs::write(
            &project_path,
            r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <OutputType>Exe</OutputType>
    </PropertyGroup>
</Project>"#,
        )
        .unwrap();

        let project = Project::load_from_path(&project_path).unwrap();
        assert_eq!(project.target_framework, "net8.0");
        assert_eq!(project.project_type, ProjectType::ConsoleApplication);
    }

    #[test]
    fn test_project_target_framework_overrides_directory_build_props() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Directory.Build.props has net6.0
        fs::write(
            temp_dir.path().join("Directory.Build.props"),
            r"
<Project>
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
    </PropertyGroup>
</Project>",
        )
        .unwrap();

        // Project file has net8.0 - Directory.Build.props should be ignored
        let project_path = temp_dir.path().join("MyProject.csproj");
        fs::write(
            &project_path,
            r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>net8.0</TargetFramework>
    </PropertyGroup>
</Project>"#,
        )
        .unwrap();

        let project = Project::load_from_path(&project_path).unwrap();
        // Project file's TargetFramework is used, Directory.Build.props is not consulted
        assert_eq!(project.target_framework, "net8.0");
    }

    #[test]
    fn test_directory_build_props_walks_up_directory_tree() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Put Directory.Build.props at root
        fs::write(
            temp_dir.path().join("Directory.Build.props"),
            r"
<Project>
    <PropertyGroup>
        <TargetFramework>net9.0</TargetFramework>
    </PropertyGroup>
</Project>",
        )
        .unwrap();

        // Create deeply nested project
        let project_dir = temp_dir.path().join("src").join("MyProject");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.join("MyProject.csproj");
        fs::write(
            &project_path,
            r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <OutputType>Exe</OutputType>
    </PropertyGroup>
</Project>"#,
        )
        .unwrap();

        let project = Project::load_from_path(&project_path).unwrap();
        assert_eq!(project.target_framework, "net9.0");
        assert_eq!(project.project_type, ProjectType::ConsoleApplication);
    }

    #[test]
    fn test_malformed_directory_build_props_returns_error() {
        let temp_dir = tempfile::tempdir().unwrap();

        fs::write(
            temp_dir.path().join("Directory.Build.props"),
            "not valid xml",
        )
        .unwrap();

        let project_path = temp_dir.path().join("MyProject.csproj");
        fs::write(
            &project_path,
            r#"
<Project Sdk="Microsoft.NET.Sdk">
</Project>"#,
        )
        .unwrap();

        let result = Project::load_from_path(&project_path);
        assert_matches!(result, Err(LoadError::XmlParseDirectoryBuildProps(_)));
    }

    #[test]
    fn test_directory_build_props_with_multiple_property_groups() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Directory.Build.props with multiple PropertyGroups
        fs::write(
            temp_dir.path().join("Directory.Build.props"),
            r"
<Project>
    <PropertyGroup>
        <TargetFramework>net6.0</TargetFramework>
    </PropertyGroup>
    <PropertyGroup>
        <TargetFramework>net8.0</TargetFramework>
    </PropertyGroup>
</Project>",
        )
        .unwrap();

        let project_path = temp_dir.path().join("MyProject.csproj");
        fs::write(
            &project_path,
            r#"
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <OutputType>Exe</OutputType>
    </PropertyGroup>
</Project>"#,
        )
        .unwrap();

        let project = Project::load_from_path(&project_path).unwrap();
        // Last PropertyGroup wins
        assert_eq!(project.target_framework, "net8.0");
    }

    #[test]
    fn test_directory_build_props_read_io_error() {
        use std::fs::Permissions;
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().unwrap();

        // Create Directory.Build.props file and make it unreadable
        let props_path = temp_dir.path().join("Directory.Build.props");
        fs::write(
            &props_path,
            r"
<Project>
    <PropertyGroup>
        <TargetFramework>net8.0</TargetFramework>
    </PropertyGroup>
</Project>",
        )
        .unwrap();

        // Make the file unreadable (mode 000)
        fs::set_permissions(&props_path, Permissions::from_mode(0o000)).unwrap();

        let project_path = temp_dir.path().join("MyProject.csproj");
        fs::write(
            &project_path,
            r#"
<Project Sdk="Microsoft.NET.Sdk">
</Project>"#,
        )
        .unwrap();

        let result = Project::load_from_path(&project_path);

        // Restore permissions for cleanup
        let _ = fs::set_permissions(&props_path, Permissions::from_mode(0o644));

        assert_matches!(result, Err(LoadError::ReadDirectoryBuildProps(_)));
    }
}
