use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn find_single_file_with_extensions(
    dir: &Path,
    extensions: &[&str],
) -> io::Result<Result<Option<PathBuf>, Vec<PathBuf>>> {
    let files = find_files_with_extensions(dir, extensions)?;

    match files.as_slice() {
        [] => Ok(Ok(None)),
        [single] => Ok(Ok(Some(single.clone()))),
        _ => Ok(Err(files)),
    }
}

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
    fn test_find_single_file_with_extensions_returns_single() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        File::create(base_path.join("test.csproj")).unwrap();
        File::create(base_path.join("README.md")).unwrap();

        let result = find_single_file_with_extensions(temp_dir.path(), &["csproj"])
            .unwrap()
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().file_name().unwrap(), "test.csproj");
    }

    #[test]
    fn test_find_single_file_with_extensions_returns_none_when_no_files() {
        let temp_dir = TempDir::new().unwrap();
        let result = find_single_file_with_extensions(temp_dir.path(), &["csproj"])
            .unwrap()
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_single_file_with_extensions_returns_error_on_multiple() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        File::create(base_path.join("test1.csproj")).unwrap();
        File::create(base_path.join("test2.vbproj")).unwrap();

        let result =
            find_single_file_with_extensions(temp_dir.path(), &["csproj", "vbproj"]).unwrap();
        assert!(matches!(result, Err(ref files) if files.len() == 2));
    }

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
}
