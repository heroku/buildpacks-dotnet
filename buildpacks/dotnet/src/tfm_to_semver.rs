use semver::VersionReq;
use std::str::FromStr;

#[derive(Debug)]
pub(crate) enum ParseTargetFrameworkError {
    InvalidFormat,
    InvalidVersion(semver::Error),
    UnsupportedOSTfm,
}

/// Parses a .NET Target Framework Moniker (TFM) and converts it into a `semver::VersionReq`.
/// It supports only .NET 6.0 and higher TFMs and rejects any OS-specific TFMs.
///
/// # Arguments
///
/// * `tfm` - A string slice that holds the TFM to be parsed.
///
/// # Returns
///
/// * `Ok(VersionReq)` - If the TFM is valid and supported.
/// * `Err(ParseTargetFrameworkError)` - If the TFM is invalid, has an unsupported format, or specifies an OS version.
/// ```
/// use tfm_to_semver::parse_target_framework;
/// use semver::VersionReq;
///
/// let version_req = parse_target_framework("net6.0").unwrap();
/// assert_eq!(version_req.to_string(), "^6.0");
/// ```
pub(crate) fn parse_target_framework(tfm: &str) -> Result<VersionReq, ParseTargetFrameworkError> {
    let valid_prefixes = ["net"];

    // Ensure the TFM is at least 4 characters long to avoid panicking
    if tfm.len() < 4 {
        return Err(ParseTargetFrameworkError::InvalidFormat);
    }

    // Safely extract the prefix and the rest of the TFM
    let prefix = &tfm[..3];
    let rest = &tfm[3..];

    // Check if the TFM starts with a valid prefix and is not empty
    if !valid_prefixes.contains(&prefix) || rest.is_empty() {
        return Err(ParseTargetFrameworkError::InvalidFormat);
    }

    // Split the TFM into base version and OS-specific parts
    let parts: Vec<&str> = rest.split('-').collect();
    if parts.len() > 1 {
        return Err(ParseTargetFrameworkError::UnsupportedOSTfm);
    }

    // Extract the numeric version part
    let version_part = parts[0]
        .split('.')
        .filter(|part| part.chars().all(char::is_numeric))
        .collect::<Vec<&str>>()
        .join(".");

    if version_part.is_empty() || !rest.chars().all(|c| c.is_numeric() || c == '.') {
        return Err(ParseTargetFrameworkError::InvalidFormat);
    }

    // Construct the VersionReq from the extracted version part
    VersionReq::from_str(&format!("^{version_part}"))
        .map_err(ParseTargetFrameworkError::InvalidVersion)
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::VersionReq;

    #[test]
    fn test_parse_valid_tfm_net6_0() {
        let tfm = "net6.0";
        let expected = VersionReq::from_str("^6.0").unwrap();
        assert_eq!(parse_target_framework(tfm).unwrap(), expected);
    }

    #[test]
    fn test_parse_valid_tfm_net7_0() {
        let tfm = "net7.0";
        let expected = VersionReq::from_str("^7.0").unwrap();
        assert_eq!(parse_target_framework(tfm).unwrap(), expected);
    }

    #[test]
    fn test_parse_valid_tfm_net8_0() {
        let tfm = "net8.0";
        let expected = VersionReq::from_str("^8.0").unwrap();
        assert_eq!(parse_target_framework(tfm).unwrap(), expected);
    }

    #[test]
    fn test_parse_invalid_tfm_empty() {
        let tfm = "";
        assert!(matches!(
            parse_target_framework(tfm),
            Err(ParseTargetFrameworkError::InvalidFormat)
        ));
    }

    #[test]
    fn test_parse_invalid_tfm_non_numeric() {
        let tfm = "netcoreapp";
        assert!(matches!(
            parse_target_framework(tfm),
            Err(ParseTargetFrameworkError::InvalidFormat)
        ));
    }

    #[test]
    fn test_parse_invalid_tfm_malformed_version() {
        let tfm = "net6.x";
        assert!(matches!(
            parse_target_framework(tfm),
            Err(ParseTargetFrameworkError::InvalidFormat)
        ));
    }

    #[test]
    fn test_parse_unsupported_os_tfm() {
        let tfm = "net6.0-ios15.0";
        assert!(matches!(
            parse_target_framework(tfm),
            Err(ParseTargetFrameworkError::UnsupportedOSTfm)
        ));
    }
}
