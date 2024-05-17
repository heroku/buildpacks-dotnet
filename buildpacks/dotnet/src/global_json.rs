use semver::VersionReq;
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum GlobalJsonError {
    #[error("failed to parse JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error("failed to parse version requirement: {0}")]
    VersionReqParseError(#[from] semver::Error),
}

/// Represents the root structure of a global.json file.
#[derive(Deserialize)]
struct GlobalJsonRoot {
    sdk: SdkConfig,
}

/// Represents the SDK configuration in a global.json file.
#[derive(Deserialize)]
struct SdkConfig {
    version: String,
    #[serde(rename = "rollForward")]
    roll_forward: Option<String>,
}

/// Constructs a `VersionReq` based on an `SdkConfig`.
///
/// # Arguments
///
/// * `sdk_config` - The SDK configuration from global.json.
///
/// # Returns
///
/// A `VersionReq` constructed based on the provided version and rollForward value.
///
/// TODO: Factor in pre-release logic
fn construct_version_req(sdk_config: &SdkConfig) -> Result<VersionReq, semver::Error> {
    let version = &sdk_config.version;
    let roll_forward = sdk_config.roll_forward.as_deref();
    match roll_forward {
        Some("patch" | "latestPatch") => VersionReq::parse(&format!("~{version}")),
        Some("feature" | "latestFeature") => {
            let parts: Vec<&str> = version.split('.').collect();
            if parts.len() > 2 {
                VersionReq::parse(&format!("~{}.{}", parts[0], parts[1]))
            } else {
                VersionReq::parse(&format!("~{version}"))
            }
        }
        Some("minor" | "latestMinor") => {
            let parts: Vec<&str> = version.split('.').collect();
            if parts.len() > 1 {
                VersionReq::parse(&format!("^{}.{}", parts[0], parts[1]))
            } else {
                VersionReq::parse(&format!("^{version}"))
            }
        }
        Some("major" | "latestMajor") => VersionReq::parse("*"),
        Some("disable") => VersionReq::parse(&format!("={version}")),
        _ => VersionReq::parse(version),
    }
}

/// Parses global.json contents and returns a `VersionReq`.
///
/// # Arguments
///
/// * `contents` - The contents of the global.json file as a `&str`.
///
/// # Returns
///
/// A `VersionReq` constructed based on the provided version and rollForward value.
///
/// TODO: Parse pre-release information
pub(crate) fn parse_global_json(contents: &str) -> Result<VersionReq, GlobalJsonError> {
    let root: GlobalJsonRoot = serde_json::from_str(contents)?;
    construct_version_req(&root.sdk).map_err(GlobalJsonError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construct_version_req() {
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
                expected: "~6.0.100",
            },
            TestCase {
                version: "6.0.100",
                roll_forward: Some("latestPatch"),
                expected: "~6.0.100",
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
                version: "6.0.100",
                roll_forward: None,
                expected: "6.0.100",
            },
        ];

        for case in &test_cases {
            let sdk_config = SdkConfig {
                version: case.version.to_string(),
                roll_forward: case.roll_forward.map(ToString::to_string),
            };
            let result = construct_version_req(&sdk_config).unwrap();
            let expected = VersionReq::parse(case.expected).unwrap();
            assert_eq!(result, expected, "Failed for case: {case:?}");
        }
    }

    #[test]
    fn test_parse_global_json() {
        let json_content = r#"
        {
            "sdk": {
                "version": "6.0.100",
                "rollForward": "latestMinor"
            }
        }
        "#;

        let result = parse_global_json(json_content);
        assert!(result.is_ok());

        let version_req = result.unwrap();
        assert_eq!(version_req, VersionReq::parse("^6.0").unwrap());
    }

    #[test]
    fn test_parse_global_json_without_rollforward() {
        let json_content = r#"
        {
            "sdk": {
                "version": "6.0.100"
            }
        }
        "#;

        let result = parse_global_json(json_content);
        assert!(result.is_ok());

        let version_req = result.unwrap();
        assert_eq!(version_req, VersionReq::parse("6.0.100").unwrap());
    }
}
