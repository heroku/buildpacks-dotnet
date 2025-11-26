use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn find_files_with_extensions(
    dir: &Path,
    extensions: &[&str],
) -> Result<Vec<PathBuf>, io::Error> {
    let files = fs_err::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| extensions.contains(&ext))
        })
        .collect();
    Ok(files)
}

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
    fn test_find_files_with_extensions_io_error() {
        // Test with a path that doesn't exist, which should cause an IO error
        let nonexistent_path = std::path::PathBuf::from("/nonexistent/directory/that/does/not/exist");
        let result = find_files_with_extensions(&nonexistent_path, &["csproj"]);
        assert!(result.is_err());
    }
}
