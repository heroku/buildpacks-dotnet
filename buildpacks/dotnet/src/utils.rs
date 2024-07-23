use std::fs;
use std::path::Path;

pub(crate) fn copy_recursively<P: AsRef<Path>>(src: P, dst: P) -> std::io::Result<()> {
    if src.as_ref().is_dir() {
        fs::create_dir_all(dst.as_ref())?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.as_ref().join(entry.file_name());

            copy_recursively(&src_path, &dst_path)?;
        }
    } else {
        fs::copy(src, dst)?;
    }
    Ok(())
}

/// Convert a [`libcnb::Env`] to a sorted vector of key-value string slice tuples, for easier
/// testing of the environment variables set in the buildpack layers.
#[cfg(test)]
pub(crate) fn environment_as_sorted_vector(environment: &libcnb::Env) -> Vec<(&str, &str)> {
    let mut result: Vec<(&str, &str)> = environment
        .iter()
        .map(|(k, v)| (k.to_str().unwrap(), v.to_str().unwrap()))
        .collect();

    result.sort_by_key(|kv| kv.0);
    result
}
