use semver::{Version, VersionReq};
use serde::Deserialize;
use std::convert::TryFrom;
use std::str::FromStr;

/// Represents the root structure of a global.json file.
#[derive(Deserialize)]
pub(crate) struct GlobalJson {
    pub(crate) sdk: Option<SdkConfig>,
}

/// Represents the SDK configuration in a global.json file.
#[derive(Deserialize)]
pub(crate) struct SdkConfig {
    version: String,
    #[serde(rename = "rollForward")]
    roll_forward: Option<String>,
}

impl FromStr for GlobalJson {
    type Err = serde_json::Error;

    fn from_str(contents: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<GlobalJson>(contents)
    }
}

/// Represents the rollForward policy for SDK version selection.
/// See <https://learn.microsoft.com/en-us/dotnet/core/tools/global-json#rollforward>
#[derive(Debug, PartialEq, Default)]
enum RollForwardPolicy {
    #[default]
    Patch,
    LatestPatch,
    Feature,
    LatestFeature,
    Minor,
    LatestMinor,
    Major,
    LatestMajor,
    Disable,
}

impl FromStr for RollForwardPolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "patch" => Ok(Self::Patch),
            "latestPatch" => Ok(Self::LatestPatch),
            "feature" => Ok(Self::Feature),
            "latestFeature" => Ok(Self::LatestFeature),
            "minor" => Ok(Self::Minor),
            "latestMinor" => Ok(Self::LatestMinor),
            "major" => Ok(Self::Major),
            "latestMajor" => Ok(Self::LatestMajor),
            "disable" => Ok(Self::Disable),
            _ => Err(s.to_string()),
        }
    }
}

#[derive(Debug)]
pub(crate) enum SdkConfigError {
    InvalidVersion(semver::Error),
    InvalidRollForward(String),
}

impl TryFrom<SdkConfig> for VersionReq {
    type Error = SdkConfigError;

    // TODO: Factor in pre-release logic
    fn try_from(sdk_config: SdkConfig) -> Result<Self, Self::Error> {
        let version_str = sdk_config.version.as_ref();
        // Parse version to ensure we have valid components to work with
        let version = Version::parse(version_str).map_err(SdkConfigError::InvalidVersion)?;

        // Default policy is `patch`, see https://learn.microsoft.com/en-us/dotnet/core/tools/global-json#matching-rules
        let policy_str = sdk_config.roll_forward.as_deref().unwrap_or("patch");
        let policy =
            RollForwardPolicy::from_str(policy_str).map_err(SdkConfigError::InvalidRollForward)?;

        let version_req_str = match policy {
            RollForwardPolicy::Patch | RollForwardPolicy::LatestPatch => {
                // Feature band logic: 6.0.1xx matches 6.0.1xx, but not 6.0.2xx.
                // See https://learn.microsoft.com/en-us/dotnet/core/tools/global-json#rollforward
                let patch = version.patch;
                let feature_band_start = (patch / 100) * 100;
                let feature_band_end = feature_band_start + 100;

                // If the user requested a pre-release (e.g., 6.0.100-rc.1),
                // we must allow pre-releases in the lower bound.
                // Using an exact comparator for the lower bound handles this best.
                format!(
                    ">={}, <{}.{}.{}",
                    version_str, // Use full string (6.0.100-rc.1) to include pre-release
                    version.major,
                    version.minor,
                    feature_band_end
                )
            }
            RollForwardPolicy::Feature | RollForwardPolicy::LatestFeature => {
                format!("~{}.{}", version.major, version.minor)
            }
            RollForwardPolicy::Minor | RollForwardPolicy::LatestMinor => {
                format!("^{}.{}", version.major, version.minor)
            }
            RollForwardPolicy::Major | RollForwardPolicy::LatestMajor => "*".to_string(),
            RollForwardPolicy::Disable => format!("={version_str}"),
        };
        VersionReq::parse(&version_req_str).map_err(SdkConfigError::InvalidVersion)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::VersionReq;
    use std::str::FromStr;

    #[test]
    fn test_construct_version_req_from_sdk() {
        #[derive(Debug)]
        struct TestCase {
            version: &'static str,
            roll_forward: Option<&'static str>,
            expected: &'static str,
        }

        let test_cases = [
            TestCase {
                version: "6.0.100",
                roll_forward: Some("patch"),
                expected: ">=6.0.100, <6.0.200",
            },
            TestCase {
                version: "6.0.100",
                roll_forward: Some("latestPatch"),
                expected: ">=6.0.100, <6.0.200",
            },
            TestCase {
                version: "6.0.201",
                roll_forward: Some("patch"),
                expected: ">=6.0.201, <6.0.300",
            },
            TestCase {
                version: "6.0.201-rc.1.12345.1",
                roll_forward: Some("patch"),
                expected: ">=6.0.201-rc.1.12345.1, <6.0.300",
            },
            TestCase {
                version: "6.0.100",
                roll_forward: Some("feature"),
                expected: "~6.0",
            },
            TestCase {
                version: "6.0.100",
                roll_forward: Some("latestFeature"),
                expected: "~6.0",
            },
            TestCase {
                version: "6.0.100",
                roll_forward: Some("minor"),
                expected: "^6.0",
            },
            TestCase {
                version: "6.0.100",
                roll_forward: Some("latestMinor"),
                expected: "^6.0",
            },
            TestCase {
                version: "6.0.100",
                roll_forward: Some("major"),
                expected: "*",
            },
            TestCase {
                version: "6.0.100",
                roll_forward: Some("latestMajor"),
                expected: "*",
            },
            TestCase {
                version: "6.0.100",
                roll_forward: Some("disable"),
                expected: "=6.0.100",
            },
            TestCase {
                version: "6.0.100-rc.1.12345.1",
                roll_forward: Some("disable"),
                expected: "=6.0.100-rc.1.12345.1",
            },
            TestCase {
                version: "6.0.100",
                roll_forward: None,
                expected: ">=6.0.100, <6.0.200",
            },
            TestCase {
                version: "6.0.100-rc.1.12345.1",
                roll_forward: None,
                expected: ">=6.0.100-rc.1.12345.1, <6.0.200",
            },
        ];

        for case in &test_cases {
            let sdk_config = SdkConfig {
                version: case.version.to_string(),
                roll_forward: case.roll_forward.map(ToString::to_string),
            };
            let result = VersionReq::try_from(sdk_config).unwrap();
            assert_eq!(result.to_string(), case.expected);
            assert!(result.matches(&Version::parse(case.version).unwrap()));
        }
    }

    #[test]
    fn test_parse_global_json_with_sdk() {
        let json_content = r#"
        {
            "sdk": {
                "version": "6.0.100",
                "rollForward": "latestMinor"
            }
        }
        "#;

        let global_json = GlobalJson::from_str(json_content).unwrap();
        let version_req = VersionReq::try_from(global_json.sdk.unwrap()).unwrap();
        assert_eq!(version_req, VersionReq::parse("^6.0").unwrap());
    }

    #[test]
    fn test_parse_empty_global_json() {
        let json_content = r"
        {
        }
        ";
        let global_json = GlobalJson::from_str(json_content).unwrap();
        assert!(global_json.sdk.is_none());
    }

    #[test]
    fn test_invalid_sdk_version() {
        let sdk_config = SdkConfig {
            version: "invalid-version".to_string(),
            roll_forward: None,
        };
        let result = VersionReq::try_from(sdk_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_incomplete_sdk_version() {
        let sdk_config = SdkConfig {
            version: "6.0".to_string(),
            roll_forward: None,
        };
        let result = VersionReq::try_from(sdk_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_roll_forward_policy() {
        let sdk_config = SdkConfig {
            version: "6.0.100".to_string(),
            roll_forward: Some("invalid".to_string()),
        };
        let result = VersionReq::try_from(sdk_config);
        assert_matches!(result, Err(SdkConfigError::InvalidRollForward(p)) if p == "invalid");
    }

    #[test]
    fn test_roll_forward_policy_from_str_valid() {
        let test_cases = [
            ("patch", RollForwardPolicy::Patch),
            ("latestPatch", RollForwardPolicy::LatestPatch),
            ("feature", RollForwardPolicy::Feature),
            ("latestFeature", RollForwardPolicy::LatestFeature),
            ("minor", RollForwardPolicy::Minor),
            ("latestMinor", RollForwardPolicy::LatestMinor),
            ("major", RollForwardPolicy::Major),
            ("latestMajor", RollForwardPolicy::LatestMajor),
            ("disable", RollForwardPolicy::Disable),
        ];

        for (input, expected) in test_cases {
            let result = RollForwardPolicy::from_str(input);
            assert_eq!(result.unwrap(), expected);
        }
    }

    #[test]
    fn test_roll_forward_policy_from_str_invalid() {
        let invalid_cases = [
            "invalid",
            "Patch",
            "latestpatch",
            "latestMinorr",
            "",
            " patch ",
        ];

        for input in &invalid_cases {
            let result = RollForwardPolicy::from_str(input);
            assert_matches!(result, Err(error) if error == input);
        }
    }

    #[test]
    fn test_roll_forward_policy_default() {
        assert_eq!(RollForwardPolicy::default(), RollForwardPolicy::Patch);
    }
}
