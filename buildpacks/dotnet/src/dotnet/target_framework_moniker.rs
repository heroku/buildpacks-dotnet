use semver::VersionReq;
use std::convert::TryFrom;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub(crate) enum ParseTargetFrameworkError {
    InvalidFormat(String),
    UnsupportedOSTfm(String),
}

#[derive(Debug, PartialEq)]
pub(crate) struct TargetFrameworkMoniker {
    pub(crate) version_part: String,
}

const SUPPORTED_PREFIX: &str = "net";

impl FromStr for TargetFrameworkMoniker {
    type Err = ParseTargetFrameworkError;

    fn from_str(tfm: &str) -> Result<Self, Self::Err> {
        if !tfm.starts_with(SUPPORTED_PREFIX) {
            return Err(ParseTargetFrameworkError::InvalidFormat(tfm.to_string()));
        }

        let rest = &tfm[SUPPORTED_PREFIX.len()..];

        let parts: Vec<&str> = rest.split('-').collect();
        if parts.len() > 1 {
            return Err(ParseTargetFrameworkError::UnsupportedOSTfm(tfm.to_string()));
        }

        let version_part = parts[0]
            .split('.')
            .filter(|part| part.chars().all(char::is_numeric))
            .collect::<Vec<&str>>()
            .join(".");

        if version_part.is_empty() || !rest.chars().all(|c| c.is_numeric() || c == '.') {
            return Err(ParseTargetFrameworkError::InvalidFormat(tfm.to_string()));
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
    fn test_parse_net6_0() {
        let tfm = "net6.0";
        let target_framework = tfm.parse::<TargetFrameworkMoniker>().unwrap();
        let expected = VersionReq::from_str("^6.0").unwrap();
        assert_eq!(VersionReq::try_from(&target_framework).unwrap(), expected);
    }

    #[test]
    fn test_parse_net7_0() {
        let tfm = "net7.0";
        let target_framework = tfm.parse::<TargetFrameworkMoniker>().unwrap();
        let expected = VersionReq::from_str("^7.0").unwrap();
        assert_eq!(VersionReq::try_from(&target_framework).unwrap(), expected);
    }

    #[test]
    fn test_parse_net8_0() {
        let tfm = "net8.0";
        let target_framework = tfm.parse::<TargetFrameworkMoniker>().unwrap();
        let expected = VersionReq::from_str("^8.0").unwrap();
        assert_eq!(VersionReq::try_from(&target_framework).unwrap(), expected);
    }

    #[test]
    fn test_parse_invalid_empty() {
        let tfm = String::new();
        assert_eq!(
            tfm.parse::<TargetFrameworkMoniker>(),
            Err(ParseTargetFrameworkError::InvalidFormat(tfm))
        );
    }

    #[test]
    fn test_parse_invalid_non_numeric() {
        let tfm = String::from("netcoreapp");
        assert_eq!(
            tfm.parse::<TargetFrameworkMoniker>(),
            Err(ParseTargetFrameworkError::InvalidFormat(tfm))
        );
    }

    #[test]
    fn test_parse_invalid_malformed_version() {
        let tfm = "net6.x".to_string();
        assert_eq!(
            tfm.parse::<TargetFrameworkMoniker>(),
            Err(ParseTargetFrameworkError::InvalidFormat(tfm))
        );
    }

    #[test]
    fn test_parse_unsupported_os() {
        let tfm = "net6.0-ios15.0".to_string();
        assert_eq!(
            tfm.parse::<TargetFrameworkMoniker>(),
            Err(ParseTargetFrameworkError::UnsupportedOSTfm(tfm))
        );
    }
}
