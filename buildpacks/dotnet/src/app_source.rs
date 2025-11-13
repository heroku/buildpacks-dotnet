use crate::detect;
use crate::dotnet::project::{LoadError as ProjectLoadError, Project};
use crate::dotnet::solution::{LoadError as SolutionLoadError, Solution};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) enum DiscoveryError {
    DetectionIoError(io::Error),
    MultipleSolutionFiles(Vec<PathBuf>),
    MultipleProjectFiles(Vec<PathBuf>),
    MultipleFileBasedApps(Vec<PathBuf>),
    UnrecognizedAppExtension(PathBuf),
    InvalidPath(PathBuf),
    NoAppFound,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AppSource {
    Solution(PathBuf),
    Project(PathBuf),
    FileBasedApp(PathBuf),
}

impl AppSource {
    fn from_dir(dir_path: &Path) -> Result<Self, DiscoveryError> {
        type Finder = fn(&Path) -> io::Result<Vec<PathBuf>>;
        type Builder = fn(PathBuf) -> AppSource;
        type OnMultipleHandler = fn(Vec<PathBuf>) -> DiscoveryError;

        let strategies: &[(Finder, Builder, OnMultipleHandler)] = &[
            (
                detect::solution_file_paths,
                AppSource::Solution,
                DiscoveryError::MultipleSolutionFiles,
            ),
            (
                detect::project_file_paths,
                AppSource::Project,
                DiscoveryError::MultipleProjectFiles,
            ),
            (
                detect::file_based_app_paths,
                AppSource::FileBasedApp,
                DiscoveryError::MultipleFileBasedApps,
            ),
        ];
        for (finder, builder, on_multiple) in strategies {
            let paths = finder(dir_path).map_err(DiscoveryError::DetectionIoError)?;
            match paths.as_slice() {
                [] => {}
                [single] => return Ok(builder(single.clone())),
                _ => return Err(on_multiple(paths)),
            }
        }

        Err(DiscoveryError::NoAppFound)
    }

    fn from_file(file_path: &Path) -> Result<Self, DiscoveryError> {
        let file_path_buf = file_path.to_path_buf();
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| DiscoveryError::UnrecognizedAppExtension(file_path_buf.clone()))?;

        match extension.to_lowercase().as_str() {
            "sln" | "slnx" => Ok(Self::Solution(file_path_buf)),
            "csproj" | "vbproj" | "fsproj" => Ok(Self::Project(file_path_buf)),
            "cs" => Ok(Self::FileBasedApp(file_path_buf)),
            _ => Err(DiscoveryError::UnrecognizedAppExtension(file_path_buf)),
        }
    }

    pub(crate) fn source_type(&self) -> &str {
        match self {
            Self::Solution(_) => "solution",
            Self::Project(_) => "project",
            Self::FileBasedApp(_) => "file-based app",
        }
    }

    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::Solution(path) | Self::Project(path) | Self::FileBasedApp(path) => path,
        }
    }
}

impl TryFrom<&Path> for AppSource {
    type Error = DiscoveryError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        if path.is_dir() {
            Self::from_dir(path)
        } else if path.is_file() {
            Self::from_file(path)
        } else {
            Err(DiscoveryError::InvalidPath(path.to_path_buf()))
        }
    }
}

#[derive(Debug)]
pub(crate) enum LoadError {
    LoadSolution(SolutionLoadError),
    LoadProject(ProjectLoadError),
    LoadFileBasedApp(io::Error),
}

impl TryFrom<AppSource> for Solution {
    type Error = LoadError;

    fn try_from(app_source: AppSource) -> Result<Self, Self::Error> {
        match app_source {
            AppSource::Solution(path) => {
                Solution::load_from_path(&path).map_err(LoadError::LoadSolution)
            }
            AppSource::Project(path) => Project::load_from_path(&path)
                .map_err(LoadError::LoadProject)
                .map(Solution::ephemeral),
            AppSource::FileBasedApp(path) => Project::load_from_file_based_app(&path)
                .map_err(LoadError::LoadFileBasedApp)
                .map(Solution::ephemeral),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_temp_dir_with_files(files: &[&str]) -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        for file in files {
            let file_path = temp_dir.path().join(file);
            let parent = file_path.parent().unwrap();
            if !parent.exists() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&file_path, "").unwrap();
        }
        temp_dir
    }

    #[test]
    fn test_from_dir_discovers_single_solution() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.sln"]);
        let app_source = AppSource::from_dir(temp_dir.path()).unwrap();

        assert!(matches!(
            app_source,
            AppSource::Solution(ref path) if path.file_name().unwrap() == "MyApp.sln"
        ));
    }

    #[test]
    fn test_from_dir_discovers_single_slnx() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.slnx"]);
        let app_source = AppSource::from_dir(temp_dir.path()).unwrap();

        assert!(matches!(
            app_source,
            AppSource::Solution(ref path) if path.file_name().unwrap() == "MyApp.slnx"
        ));
    }

    #[test]
    fn test_from_dir_discovers_single_project() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.csproj"]);
        let app_source = AppSource::from_dir(temp_dir.path()).unwrap();

        assert!(matches!(
            app_source,
            AppSource::Project(ref path) if path.file_name().unwrap() == "MyApp.csproj"
        ));
    }

    #[test]
    fn test_from_dir_discovers_vbproj() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.vbproj"]);
        let app_source = AppSource::from_dir(temp_dir.path()).unwrap();

        assert!(matches!(
            app_source,
            AppSource::Project(ref path) if path.file_name().unwrap() == "MyApp.vbproj"
        ));
    }

    #[test]
    fn test_from_dir_discovers_fsproj() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.fsproj"]);
        let app_source = AppSource::from_dir(temp_dir.path()).unwrap();

        assert!(matches!(
            app_source,
            AppSource::Project(ref path) if path.file_name().unwrap() == "MyApp.fsproj"
        ));
    }

    #[test]
    fn test_from_dir_discovers_single_file_based_app() {
        let temp_dir = create_temp_dir_with_files(&["app.cs"]);
        let app_source = AppSource::from_dir(temp_dir.path()).unwrap();

        assert!(matches!(
            app_source,
            AppSource::FileBasedApp(ref path) if path.file_name().unwrap() == "app.cs"
        ));
    }

    #[test]
    fn test_from_dir_solution_takes_precedence_over_project() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.sln", "MyApp.csproj"]);
        let app_source = AppSource::from_dir(temp_dir.path()).unwrap();

        assert!(matches!(
            app_source,
            AppSource::Solution(ref path) if path.file_name().unwrap() == "MyApp.sln"
        ));
    }

    #[test]
    fn test_from_dir_solution_takes_precedence_over_file_based_app() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.sln", "app.cs"]);
        let app_source = AppSource::from_dir(temp_dir.path()).unwrap();

        assert!(matches!(app_source, AppSource::Solution(_)));
    }

    #[test]
    fn test_from_dir_project_takes_precedence_over_file_based_app() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.csproj", "app.cs"]);
        let app_source = AppSource::from_dir(temp_dir.path()).unwrap();

        assert!(matches!(
            app_source,
            AppSource::Project(ref path) if path.file_name().unwrap() == "MyApp.csproj"
        ));
    }

    #[test]
    fn test_from_dir_no_app_found_in_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = AppSource::from_dir(temp_dir.path());

        assert!(matches!(result, Err(DiscoveryError::NoAppFound)));
    }

    #[test]
    fn test_from_dir_multiple_solutions_error() {
        let temp_dir = create_temp_dir_with_files(&["App1.sln", "App2.sln"]);
        let result = AppSource::from_dir(temp_dir.path());

        assert!(matches!(
            result,
            Err(DiscoveryError::MultipleSolutionFiles(ref paths)) if paths.len() == 2
        ));
    }

    #[test]
    fn test_from_dir_multiple_projects_error() {
        let temp_dir = create_temp_dir_with_files(&["App1.csproj", "App2.csproj"]);
        let result = AppSource::from_dir(temp_dir.path());

        assert!(matches!(
            result,
            Err(DiscoveryError::MultipleProjectFiles(ref paths)) if paths.len() == 2
        ));
    }

    #[test]
    fn test_from_dir_multiple_file_based_apps_error() {
        let temp_dir = create_temp_dir_with_files(&["app1.cs", "app2.cs"]);
        let result = AppSource::from_dir(temp_dir.path());

        assert!(matches!(
            result,
            Err(DiscoveryError::MultipleFileBasedApps(ref paths)) if paths.len() == 2
        ));
    }

    #[test]
    fn test_from_file_discovers_solution() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.sln"]);
        let app_source = AppSource::from_file(temp_dir.path().join("MyApp.sln").as_path()).unwrap();
        assert!(matches!(
            app_source,
            AppSource::Solution(ref path) if path.file_name().unwrap() == "MyApp.sln"
        ));
    }

    #[test]
    fn test_from_file_discovers_slnx() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.slnx"]);
        let app_source = AppSource::from_file(temp_dir.path().join("MyApp.slnx").as_path()).unwrap();
        assert!(matches!(
            app_source,
            AppSource::Solution(ref path) if path.file_name().unwrap() == "MyApp.slnx"
        ));
    }

    #[test]
    fn test_from_file_discovers_csproj() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.csproj"]);
        let app_source =
            AppSource::from_file(temp_dir.path().join("MyApp.csproj").as_path()).unwrap();
        assert!(matches!(
            app_source,
            AppSource::Project(ref path) if path.file_name().unwrap() == "MyApp.csproj"
        ));
    }

    #[test]
    fn test_from_file_discovers_vbproj() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.vbproj"]);
        let app_source =
            AppSource::from_file(temp_dir.path().join("MyApp.vbproj").as_path()).unwrap();
        assert!(matches!(
            app_source,
            AppSource::Project(ref path) if path.file_name().unwrap() == "MyApp.vbproj"
        ));
    }

    #[test]
    fn test_from_file_discovers_fsproj() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.fsproj"]);
        let app_source =
            AppSource::from_file(temp_dir.path().join("MyApp.fsproj").as_path()).unwrap();
        assert!(matches!(
            app_source,
            AppSource::Project(ref path) if path.file_name().unwrap() == "MyApp.fsproj"
        ));
    }

    #[test]
    fn test_from_file_discovers_cs_file() {
        let temp_dir = create_temp_dir_with_files(&["app.cs"]);
        let app_source = AppSource::from_file(temp_dir.path().join("app.cs").as_path()).unwrap();
        assert!(matches!(
            app_source,
            AppSource::FileBasedApp(ref path) if path.file_name().unwrap() == "app.cs"
        ));
    }

    #[test]
    fn test_from_file_with_nested_path() {
        let temp_dir = create_temp_dir_with_files(&["src/MyApp/MyApp.csproj"]);
        let app_source =
            AppSource::from_file(temp_dir.path().join("src/MyApp/MyApp.csproj").as_path()).unwrap();
        assert!(matches!(app_source, AppSource::Project(_)));
    }

    #[test]
    fn test_from_file_invalid_extension() {
        let temp_dir = create_temp_dir_with_files(&["MyApp.txt"]);
        let result = AppSource::from_file(temp_dir.path().join("MyApp.txt").as_path());
        assert!(matches!(
            result,
            Err(DiscoveryError::UnrecognizedAppExtension(_))
        ));
    }

    #[test]
    fn test_from_file_no_extension() {
        let temp_dir = create_temp_dir_with_files(&["MyApp"]);
        let result = AppSource::from_file(temp_dir.path().join("MyApp").as_path());
        assert!(matches!(
            result,
            Err(DiscoveryError::UnrecognizedAppExtension(_))
        ));
    }

    #[test]
    fn test_discover_from_non_existent_path() {
        let temp_dir = TempDir::new().unwrap();
        let result = AppSource::try_from(temp_dir.path().join("nonexistent.sln").as_path());
        assert!(matches!(result, Err(DiscoveryError::InvalidPath(_))));
    }

    #[test]
    fn test_try_from_app_source_creates_ephemeral_solution_for_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("MyApp.csproj");

        fs::write(
            &project_path,
            r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>
</Project>"#,
        )
        .unwrap();

        let app_source = AppSource::Project(project_path);
        let solution = Solution::try_from(app_source).unwrap();

        assert_eq!(solution.projects.len(), 1);
        assert_eq!(solution.projects[0].assembly_name, "MyApp");
    }

    #[test]
    fn test_try_from_app_source_creates_ephemeral_solution_for_file_based_app() {
        let temp_dir = TempDir::new().unwrap();
        let cs_path = temp_dir.path().join("MyApp.cs");

        fs::write(&cs_path, "Console.WriteLine(\"Hello, World!\");").unwrap();

        let app_source = AppSource::FileBasedApp(cs_path);
        let solution = Solution::try_from(app_source).unwrap();

        assert_eq!(solution.projects.len(), 1);
        assert_eq!(solution.projects[0].assembly_name, "MyApp");
    }

    #[test]
    fn test_try_from_app_source_returns_solution_for_slnx() {
        let temp_dir = TempDir::new().unwrap();
        let slnx_path = temp_dir.path().join("MySolution.slnx");
        let project_path = temp_dir.path().join("MyProject.csproj");

        fs::write(
            &slnx_path,
            r#"<Solution><Project Path="MyProject.csproj" /></Solution>"#,
        )
        .unwrap();

        fs::write(
            &project_path,
            r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>
</Project>"#,
        )
        .unwrap();

        let app_source = AppSource::Solution(slnx_path);
        let solution = Solution::try_from(app_source).unwrap();

        assert_eq!(solution.projects.len(), 1);
        assert_eq!(solution.projects[0].assembly_name, "MyProject");
    }

    #[test]
    fn test_source_type_returns_correct_description() {
        let solution = AppSource::Solution(PathBuf::from("test.sln"));
        let project = AppSource::Project(PathBuf::from("test.csproj"));
        let file_based = AppSource::FileBasedApp(PathBuf::from("test.cs"));

        assert_eq!(solution.source_type(), "solution");
        assert_eq!(project.source_type(), "project");
        assert_eq!(file_based.source_type(), "file-based app");
    }

    #[test]
    fn test_path_returns_correct_path() {
        let solution_path = PathBuf::from("/tmp/test.sln");
        let project_path = PathBuf::from("/tmp/test.csproj");
        let cs_path = PathBuf::from("/tmp/test.cs");

        let solution = AppSource::Solution(solution_path.clone());
        let project = AppSource::Project(project_path.clone());
        let file_based = AppSource::FileBasedApp(cs_path.clone());

        assert_eq!(solution.path(), solution_path.as_path());
        assert_eq!(project.path(), project_path.as_path());
        assert_eq!(file_based.path(), cs_path.as_path());
    }
}
