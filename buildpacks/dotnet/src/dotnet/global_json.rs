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

impl TryFrom<SdkConfig> for VersionReq {
    type Error = semver::Error;

    // TODO: Factor in pre-release logic
    fn try_from(sdk_config: SdkConfig) -> Result<Self, Self::Error> {
        let version_str = sdk_config.version.as_ref();
        // Parse version to ensure we have valid components to work with
        let version = Version::parse(version_str)?;

        // Default policy is `patch`, see https://learn.microsoft.com/en-us/dotnet/core/tools/global-json#matching-rules
        let policy = sdk_config.roll_forward.as_deref().unwrap_or("patch");

        let version_req_str = match policy {
            "patch" | "latestPatch" => {
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
            "feature" | "latestFeature" => {
                format!("~{}.{}", version.major, version.minor)
            }
            "minor" | "latestMinor" => {
                format!("^{}.{}", version.major, version.minor)
            }
            "major" | "latestMajor" => "*".to_string(),
            "disable" => format!("={version_str}"),
            _ => version_str.to_string(),
        };
        VersionReq::parse(&version_req_str)
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
    fn test_parse_global_json_without_sdk_rollforward() {
        let sdk_config = SdkConfig {
            version: "6.0.100".to_string(),
            roll_forward: None,
        };
        let version_req = VersionReq::try_from(sdk_config).unwrap();
        assert_eq!(
            version_req,
            VersionReq::parse(">=6.0.100, <6.0.200").unwrap()
        );
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
}
