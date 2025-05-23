use std::path::Path;

pub(crate) fn copy_recursively<P: AsRef<Path>>(src: P, dst: P) -> std::io::Result<()> {
    if src.as_ref().is_dir() {
        fs_err::create_dir_all(dst.as_ref())?;
        for entry in fs_err::read_dir(src.as_ref())? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.as_ref().join(entry.file_name());

            copy_recursively(&src_path, &dst_path)?;
        }
    } else {
        fs_err::copy(src, dst)?;
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

/// Converts an arbitrary string slice into an RFC 1123-compliant DNS label.
///
/// RFC References:
/// - RFC 1123 (section 2.1): Allows labels to start with letters or digits.
/// - RFC 1035 (section 2.3.1): Defines allowed characters (`a-z`, `0-9`, `-`), maximum 63 characters.
///
/// Implementation Details:
/// - Converts to lowercase (by convention, DNS is case-insensitive).
/// - Keeps ASCII letters, digits, and hyphens.
/// - Treats `.`, `_`, and space characters as separators, replacing them with hyphens.
/// - Discards all other characters (e.g. `!`, `@`, `&`, `*`).
/// - Collapses repeated hyphen-producing characters into a single hyphen.
/// - Removes leading/trailing hyphens.
/// - Truncates labels exceeding 63 characters.
///
/// Errors:
/// Returns an error if sanitization results in an empty label.
pub(crate) fn to_rfc1123_label(input: &str) -> Result<String, ()> {
    let mut label = String::new();

    let mut previous_char_was_hyphen = false;
    for char in input.chars().map(|c| c.to_ascii_lowercase()) {
        match char {
            'a'..='z' | '0'..='9' => {
                label.push(char);
                previous_char_was_hyphen = false;
            }
            '-' | '.' | '_' | ' ' => {
                if !previous_char_was_hyphen {
                    label.push('-');
                    previous_char_was_hyphen = true;
                }
            }
            _ => {}
        }
    }

    label = label
        .trim_matches('-')
        .chars()
        .take(63)
        .collect::<String>()
        .trim_end_matches('-')
        .to_string();
    if label.is_empty() { Err(()) } else { Ok(label) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_allows_letters_digits_hyphen() {
        assert_eq!(to_rfc1123_label("abc-123").unwrap(), "abc-123");
    }

    #[test]
    fn test_allows_leading_digits() {
        assert_eq!(to_rfc1123_label("123label").unwrap(), "123label");
    }

    #[test]
    fn test_lowercases_input() {
        assert_eq!(to_rfc1123_label("MiXeDCase").unwrap(), "mixedcase");
    }

    #[test]
    fn test_replaces_separators_with_hyphen() {
        assert_eq!(to_rfc1123_label("a.b_c d-e").unwrap(), "a-b-c-d-e");
    }

    #[test]
    fn test_removes_symbol_characters() {
        assert_eq!(to_rfc1123_label("foo!@#%^bar&*():日本").unwrap(), "foobar");
    }

    #[test]
    fn test_collapses_multiple_separator_chars() {
        assert_eq!(to_rfc1123_label("a__b..c  d").unwrap(), "a-b-c-d");
    }

    #[test]
    fn test_trims_leading_and_trailing_hyphens() {
        assert_eq!(to_rfc1123_label("--abc--").unwrap(), "abc");
        assert_eq!(to_rfc1123_label("...abc...").unwrap(), "abc");
    }

    #[test]
    fn test_truncates_to_63_characters() {
        let input = format!("a_b.c-d{}", "x".repeat(100));
        let result = to_rfc1123_label(&input).unwrap();
        assert_eq!(result.len(), 63);
    }

    #[test]
    fn test_removes_trailing_hyphen_after_truncation() {
        let input = format!("{}-aaaaaaa", "a".repeat(62));
        let result = to_rfc1123_label(&input).unwrap();
        assert_eq!(result.len(), 62);
        assert!(!result.ends_with('-'));
    }

    #[test]
    fn test_errors_on_empty_label() {
        assert!(to_rfc1123_label("").is_err());
        assert!(to_rfc1123_label("!!!").is_err());
        assert!(to_rfc1123_label("###@@@%%%").is_err());
    }

    #[test]
    fn test_copy_recursively_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_file = temp_dir.path().join("test.txt");
        let dst_file = temp_dir.path().join("copy.txt");

        fs::write(&src_file, "test content").unwrap();

        copy_recursively(&src_file, &dst_file).unwrap();

        assert!(dst_file.exists());
        assert_eq!(fs::read_to_string(&dst_file).unwrap(), "test content");
    }

    #[test]
    fn test_copy_recursively_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        let dst_dir = temp_dir.path().join("dst");

        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("file1.txt"), "file1 content").unwrap();
        fs::create_dir_all(src_dir.join("subdir")).unwrap();
        fs::write(src_dir.join("subdir").join("file2.txt"), "file2 content").unwrap();

        copy_recursively(&src_dir, &dst_dir).unwrap();

        assert!(dst_dir.exists());
        assert!(dst_dir.join("file1.txt").exists());
        assert!(dst_dir.join("subdir").exists());
        assert!(dst_dir.join("subdir").join("file2.txt").exists());
    }

    #[test]
    fn test_copy_recursively_nonexistent_source() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src = temp_dir.path().join("nonexistent");
        let dst = temp_dir.path().join("copy");

        assert!(copy_recursively(&src, &dst).is_err());
    }
}
