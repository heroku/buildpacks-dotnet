use crate::dotnet_publish_command::VerbosityLevel;

#[derive(Debug, PartialEq)]
pub(crate) struct DotnetBuildpackConfiguration {
    pub(crate) build_configuration: Option<String>,
    pub(crate) execution_environment: ExecutionEnvironment,
    pub(crate) msbuild_verbosity_level: Option<VerbosityLevel>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ExecutionEnvironment {
    Production,
    Test,
}

#[derive(Debug, PartialEq)]
pub(crate) enum DotnetBuildpackConfigurationError {
    InvalidMsbuildVerbosityLevel(String),
}

impl TryFrom<&libcnb::Env> for DotnetBuildpackConfiguration {
    type Error = DotnetBuildpackConfigurationError;

    fn try_from(env: &libcnb::Env) -> Result<Self, Self::Error> {
        Ok(Self {
            build_configuration: env
                .get("BUILD_CONFIGURATION")
                .map(|value| value.to_string_lossy().to_string()),
            execution_environment: env.get_string_lossy("CNB_EXEC_ENV").map_or(
                ExecutionEnvironment::Production,
                |value| match value.as_str() {
                    "test" => ExecutionEnvironment::Test,
                    "production" => ExecutionEnvironment::Production,
                    _ => unimplemented!("Unsupported CNB execution environment \"{value}\""),
                },
            ),
            msbuild_verbosity_level: detect_msbuild_verbosity_level(env).transpose()?,
        })
    }
}

fn detect_msbuild_verbosity_level(
    env: &libcnb::Env,
) -> Option<Result<VerbosityLevel, DotnetBuildpackConfigurationError>> {
    env.get("MSBUILD_VERBOSITY_LEVEL")
        .map(|value| value.to_string_lossy())
        .map(|value| match value.to_lowercase().as_str() {
            "q" | "quiet" => Ok(VerbosityLevel::Quiet),
            "m" | "minimal" => Ok(VerbosityLevel::Minimal),
            "n" | "normal" => Ok(VerbosityLevel::Normal),
            "d" | "detailed" => Ok(VerbosityLevel::Detailed),
            "diag" | "diagnostic" => Ok(VerbosityLevel::Diagnostic),
            _ => Err(
                DotnetBuildpackConfigurationError::InvalidMsbuildVerbosityLevel(value.to_string()),
            ),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use libcnb::Env;

    fn create_env(variables: &[(&str, &str)]) -> Env {
        let mut env = Env::new();
        for &(key, value) in variables {
            env.insert(key, value);
        }
        env
    }

    #[test]
    fn test_default_buildpack_configuration() {
        let env = create_env(&[]);
        let result = DotnetBuildpackConfiguration::try_from(&env).unwrap();

        assert_eq!(
            result,
            DotnetBuildpackConfiguration {
                build_configuration: None,
                execution_environment: ExecutionEnvironment::Production,
                msbuild_verbosity_level: None
            }
        );
    }

    #[test]
    fn test_parse_execution_environment_env_var() {
        let cases = [
            ("test", ExecutionEnvironment::Test),
            ("production", ExecutionEnvironment::Production),
        ];
        for (input, expected) in cases {
            let env = create_env(&[("CNB_EXEC_ENV", input)]);
            let result = DotnetBuildpackConfiguration::try_from(&env).unwrap();

            assert_eq!(
                result,
                DotnetBuildpackConfiguration {
                    build_configuration: None,
                    execution_environment: expected,
                    msbuild_verbosity_level: None
                }
            );
        }
    }

    #[test]
    fn test_detect_msbuild_verbosity_level() {
        let cases = [
            ("q", Ok(VerbosityLevel::Quiet)),
            ("quiet", Ok(VerbosityLevel::Quiet)),
            ("minimal", Ok(VerbosityLevel::Minimal)),
            ("m", Ok(VerbosityLevel::Minimal)),
            ("normal", Ok(VerbosityLevel::Normal)),
            ("n", Ok(VerbosityLevel::Normal)),
            ("detailed", Ok(VerbosityLevel::Detailed)),
            ("d", Ok(VerbosityLevel::Detailed)),
            ("diagnostic", Ok(VerbosityLevel::Diagnostic)),
            ("diag", Ok(VerbosityLevel::Diagnostic)),
            (
                "invalid",
                Err(
                    DotnetBuildpackConfigurationError::InvalidMsbuildVerbosityLevel(
                        "invalid".to_string(),
                    ),
                ),
            ),
        ];

        for (input, expected) in cases {
            let env = create_env(&[("MSBUILD_VERBOSITY_LEVEL", input)]);
            let result = detect_msbuild_verbosity_level(&env);
            assert_eq!(result, Some(expected));
        }
        assert!(detect_msbuild_verbosity_level(&Env::new()).is_none());
    }
}
