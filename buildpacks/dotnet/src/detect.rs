use std::fs::{self};
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn project_file_paths<P: AsRef<Path>>(dir: P) -> io::Result<Vec<PathBuf>> {
    get_files_with_extensions(dir.as_ref(), &["csproj", "vbproj", "fsproj"])
}

pub(crate) fn solution_file_paths<P: AsRef<Path>>(dir: P) -> io::Result<Vec<PathBuf>> {
    get_files_with_extensions(dir.as_ref(), &["sln"])
}

pub(crate) fn get_files_with_extensions(
    dir: &Path,
    extensions: &[&str],
) -> Result<Vec<PathBuf>, io::Error> {
    let project_files = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| extensions.contains(&ext))
        })
        .collect();
    Ok(project_files)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir, File};
    use tempfile::TempDir;

    #[test]
    fn test_find_project_files() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        File::create(base_path.join("test1.csproj")).unwrap();
        File::create(base_path.join("test2.vbproj")).unwrap();
        File::create(base_path.join("test3.fsproj")).unwrap();
        File::create(base_path.join("README.md")).unwrap();

        let project_files = project_file_paths(&temp_dir).unwrap();

        assert_eq!(3, project_files.len());
    }

    #[test]
    fn test_find_solution_files() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        File::create(base_path.join("test1.sln")).unwrap();
        File::create(base_path.join("test2.sln")).unwrap();
        File::create(base_path.join("README.md")).unwrap();

        let solution_files = solution_file_paths(&temp_dir).unwrap();

        assert_eq!(2, solution_files.len());
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
}
