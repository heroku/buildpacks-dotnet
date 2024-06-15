use std::fs::{self};
use std::io;
use std::path::{Path, PathBuf};

use crate::DotnetBuildpackError;

pub(crate) fn project_file_paths<P: AsRef<Path>>(dir: P) -> io::Result<Vec<PathBuf>> {
    get_files_with_extensions(dir.as_ref(), &["csproj", "vbproj", "fsproj"])
}

pub(crate) fn solution_file_paths<P: AsRef<Path>>(dir: P) -> io::Result<Vec<PathBuf>> {
    get_files_with_extensions(dir.as_ref(), &["sln"])
}

pub(crate) fn any_dotnet_files(dir: &Path) -> Result<bool, DotnetBuildpackError> {
    Ok(solution_file_paths(dir)
        .map_err(DotnetBuildpackError::BuildpackDetection)?
        .is_empty()
        && project_file_paths(dir)
            .map_err(DotnetBuildpackError::BuildpackDetection)?
            .is_empty())
}

fn get_files_with_extensions(dir: &Path, extensions: &[&str]) -> Result<Vec<PathBuf>, io::Error> {
    let project_files = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file()) // TODO: This returns false if there's an error)
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
    let dir = dir.as_ref();
    if !dir.is_dir() {
        return None;
    }

    let global_json_path = dir.join("global.json");
    if global_json_path.exists() && global_json_path.is_file() {
        Some(global_json_path)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempdir::TempDir;

    #[test]
    fn test_find_project_files() {
        let temp_dir = TempDir::new("dotnet-test").unwrap();
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
        let temp_dir = TempDir::new("dotnet-test").unwrap();
        let base_path = temp_dir.path();

        File::create(base_path.join("test1.sln")).unwrap();
        File::create(base_path.join("test2.sln")).unwrap();
        File::create(base_path.join("README.md")).unwrap();

        let solution_files = solution_file_paths(&temp_dir).unwrap();

        assert_eq!(2, solution_files.len());
    }

    #[test]
    fn test_global_json_file_exists() {
        let temp_dir = TempDir::new("test_global_json").unwrap();
        let global_json_path = temp_dir.path().join("global.json");

        File::create(&global_json_path).unwrap();

        let result = global_json_file(temp_dir.path());
        assert_eq!(result, Some(global_json_path));
    }

    #[test]
    fn test_global_json_file_does_not_exist() {
        let temp_dir = TempDir::new("test_dir_not_exists").unwrap();
        let result = global_json_file(temp_dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_global_json_file_is_directory() {
        let temp_dir = TempDir::new("test_dir_is_directory").unwrap();
        let global_json_path = temp_dir.path().join("global.json");
        fs::create_dir(global_json_path).unwrap();

        let result = global_json_file(temp_dir.path());
        assert_eq!(result, None);
    }
}
