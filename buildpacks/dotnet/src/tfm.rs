use semver::VersionReq;
use std::convert::TryFrom;
use std::str::FromStr;

#[derive(thiserror::Error, Debug)]
pub(crate) enum ParseTargetFrameworkError {
    #[error("Invalid target framework format")]
    InvalidFormat,
    #[error("Unsupported OS-specific target framework")]
    UnsupportedOSTfm,
}

#[derive(Debug)]
pub(crate) struct TargetFrameworkMoniker {
    pub(crate) version_part: String,
}

impl FromStr for TargetFrameworkMoniker {
    type Err = ParseTargetFrameworkError;

    fn from_str(tfm: &str) -> Result<Self, Self::Err> {
        let valid_prefixes = ["net"];

        if tfm.len() < 4 {
            return Err(ParseTargetFrameworkError::InvalidFormat);
        }

        let prefix = &tfm[..3];
        let rest = &tfm[3..];

        if !valid_prefixes.contains(&prefix) || rest.is_empty() {
            return Err(ParseTargetFrameworkError::InvalidFormat);
        }

        let parts: Vec<&str> = rest.split('-').collect();
        if parts.len() > 1 {
            return Err(ParseTargetFrameworkError::UnsupportedOSTfm);
        }

        let version_part = parts[0]
            .split('.')
            .filter(|part| part.chars().all(char::is_numeric))
            .collect::<Vec<&str>>()
            .join(".");

        if version_part.is_empty() || !rest.chars().all(|c| c.is_numeric() || c == '.') {
            return Err(ParseTargetFrameworkError::InvalidFormat);
        }

        Ok(TargetFrameworkMoniker { version_part })
    }
}

impl TryFrom<&TargetFrameworkMoniker> for VersionReq {
    type Error = semver::Error;

    fn try_from(tf: &TargetFrameworkMoniker) -> Result<Self, Self::Error> {
        VersionReq::from_str(&format!("^{}", tf.version_part))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::VersionReq;
    use std::convert::TryFrom;

    #[test]
    fn test_parse_valid_tfm_net6_0() {
        let tfm = "net6.0";
        let target_framework = tfm.parse::<TargetFrameworkMoniker>().unwrap();
        let expected = VersionReq::from_str("^6.0").unwrap();
        assert_eq!(VersionReq::try_from(&target_framework).unwrap(), expected);
    }

    #[test]
    fn test_parse_valid_tfm_net7_0() {
        let tfm = "net7.0";
        let target_framework = tfm.parse::<TargetFrameworkMoniker>().unwrap();
        let expected = VersionReq::from_str("^7.0").unwrap();
        assert_eq!(VersionReq::try_from(&target_framework).unwrap(), expected);
    }

    #[test]
    fn test_parse_valid_tfm_net8_0() {
        let tfm = "net8.0";
        let target_framework = tfm.parse::<TargetFrameworkMoniker>().unwrap();
        let expected = VersionReq::from_str("^8.0").unwrap();
        assert_eq!(VersionReq::try_from(&target_framework).unwrap(), expected);
    }

    #[test]
    fn test_parse_invalid_tfm_empty() {
        let tfm = "";
        assert!(matches!(
            tfm.parse::<TargetFrameworkMoniker>(),
            Err(ParseTargetFrameworkError::InvalidFormat)
        ));
    }

    #[test]
    fn test_parse_invalid_tfm_non_numeric() {
        let tfm = "netcoreapp";
        assert!(matches!(
            tfm.parse::<TargetFrameworkMoniker>(),
            Err(ParseTargetFrameworkError::InvalidFormat)
        ));
    }

    #[test]
    fn test_parse_invalid_tfm_malformed_version() {
        let tfm = "net6.x";
        assert!(matches!(
            tfm.parse::<TargetFrameworkMoniker>(),
            Err(ParseTargetFrameworkError::InvalidFormat)
        ));
    }

    #[test]
    fn test_parse_unsupported_os_tfm() {
        let tfm = "net6.0-ios15.0";
        assert!(matches!(
            tfm.parse::<TargetFrameworkMoniker>(),
            Err(ParseTargetFrameworkError::UnsupportedOSTfm)
        ));
    }
}
