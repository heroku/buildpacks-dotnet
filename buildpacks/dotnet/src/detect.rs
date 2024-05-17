use std::fs::{self};
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn dotnet_project_files<P: AsRef<Path>>(dir: P) -> io::Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let project_extensions = ["csproj", "vbproj", "fsproj"];
    let project_files = fs::read_dir(dir)?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file()) // TODO: This returns false if there's an error)
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| project_extensions.contains(&ext))
        })
        .collect();

    Ok(project_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempdir::TempDir;

    #[test]
    fn test_find_dotnet_project_files() {
        let temp_dir = TempDir::new("dotnet-test").unwrap();
        let base_path = temp_dir.path();

        File::create(base_path.join("test1.csproj")).unwrap();
        File::create(base_path.join("test2.vbproj")).unwrap();
        File::create(base_path.join("test3.fsproj")).unwrap();
        File::create(base_path.join("README.md")).unwrap();

        let project_files = dotnet_project_files(&temp_dir).unwrap();

        assert_eq!(3, project_files.len());
    }
}
