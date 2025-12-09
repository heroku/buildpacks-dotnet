use std::path::{Path, PathBuf};

/// Returns the path to `global.json` if it exists in the given directory.
pub(crate) fn global_json_file<P: AsRef<Path>>(dir: P) -> Option<PathBuf> {
    let path = dir.as_ref().join("global.json");
    path.is_file().then_some(path)
}

/// Returns the path to `.config/dotnet-tools.json` if it exists.
pub(crate) fn dotnet_tools_manifest_file<P: AsRef<Path>>(dir: P) -> Option<PathBuf> {
    let path = dir.as_ref().join(".config/dotnet-tools.json");
    path.is_file().then_some(path)
}

/// Returns the path to `project.toml` if it exists in the given directory.
pub(crate) fn project_toml_file<P: AsRef<Path>>(dir: P) -> Option<PathBuf> {
    let path = dir.as_ref().join("project.toml");
    path.is_file().then_some(path)
}

/// Returns the path to `Directory.Build.props` by walking up the directory tree
/// from the given starting path. Returns `None` if no such file is found
/// before reaching the filesystem root.
///
/// This follows `MSBuild`'s convention where `Directory.Build.props` files in
/// parent directories are automatically imported into projects.
///
/// The starting path can be either a file or directory.
pub(crate) fn directory_build_props_file<P: AsRef<Path>>(start_path: P) -> Option<PathBuf> {
    let path = start_path.as_ref();

    for ancestor in path.ancestors() {
        let props_path = ancestor.join("Directory.Build.props");
        if props_path.is_file() {
            return Some(props_path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File, create_dir};
    use tempfile::TempDir;

    #[test]
    fn test_global_json_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let global_json_path = temp_dir.path().join("global.json");

        File::create(&global_json_path).unwrap();

        let result = global_json_file(temp_dir.path());
        assert_eq!(result, Some(global_json_path));
    }

    #[test]
    fn test_global_json_file_does_not_exist() {
        let temp_dir = TempDir::new().unwrap();
        let result = global_json_file(temp_dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_dotnet_tools_manifest_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join(".config");
        create_dir(&config_dir).unwrap();

        let dotnet_tools_path = config_dir.join("dotnet-tools.json");
        File::create(&dotnet_tools_path).unwrap();

        let result = dotnet_tools_manifest_file(temp_dir.path());
        assert_eq!(result, Some(dotnet_tools_path));
    }

    #[test]
    fn test_dotnet_tools_manifest_file_does_not_exist() {
        let temp_dir = TempDir::new().unwrap();
        let result = dotnet_tools_manifest_file(temp_dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_global_json_file_is_directory() {
        let temp_dir = TempDir::new().unwrap();
        let global_json_path = temp_dir.path().join("global.json");
        fs::create_dir(global_json_path).unwrap();

        let result = global_json_file(temp_dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_project_toml_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let project_toml_path = temp_dir.path().join("project.toml");

        File::create(&project_toml_path).unwrap();

        let result = project_toml_file(temp_dir.path());
        assert_eq!(result, Some(project_toml_path));
    }

    #[test]
    fn test_project_toml_file_does_not_exist() {
        let temp_dir = TempDir::new().unwrap();
        let result = project_toml_file(temp_dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_project_toml_file_is_directory() {
        let temp_dir = TempDir::new().unwrap();
        let project_toml_path = temp_dir.path().join("project.toml");
        fs::create_dir(project_toml_path).unwrap();

        let result = project_toml_file(temp_dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_directory_build_props_file_exists_in_same_dir() {
        let temp_dir = TempDir::new().unwrap();
        let props_path = temp_dir.path().join("Directory.Build.props");
        File::create(&props_path).unwrap();

        let result = directory_build_props_file(temp_dir.path());
        assert_eq!(result, Some(props_path));
    }

    #[test]
    fn test_directory_build_props_file_does_not_exist() {
        let temp_dir = TempDir::new().unwrap();
        let result = directory_build_props_file(temp_dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_directory_build_props_file_walks_up_tree() {
        let temp_dir = TempDir::new().unwrap();
        let props_path = temp_dir.path().join("Directory.Build.props");
        File::create(&props_path).unwrap();

        let nested_dir = temp_dir.path().join("src").join("project");
        fs::create_dir_all(&nested_dir).unwrap();

        let result = directory_build_props_file(&nested_dir);
        assert_eq!(result, Some(props_path));
    }

    #[test]
    fn test_directory_build_props_file_is_directory() {
        let temp_dir = TempDir::new().unwrap();
        let props_path = temp_dir.path().join("Directory.Build.props");
        fs::create_dir(&props_path).unwrap();

        let result = directory_build_props_file(temp_dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_directory_build_props_file_finds_nearest() {
        let temp_dir = TempDir::new().unwrap();

        // Create props file at root
        let root_props = temp_dir.path().join("Directory.Build.props");
        File::create(&root_props).unwrap();

        // Create nested directory with its own props file
        let nested_dir = temp_dir.path().join("src");
        fs::create_dir(&nested_dir).unwrap();
        let nested_props = nested_dir.join("Directory.Build.props");
        File::create(&nested_props).unwrap();

        // Should find the nearest one (in nested_dir)
        let result = directory_build_props_file(&nested_dir);
        assert_eq!(result, Some(nested_props));
    }
}
