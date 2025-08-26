use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub(crate) struct DotnetBuildpackConfiguration {
    pub(crate) build_configuration: Option<String>,
    pub(crate) execution_environment: ExecutionEnvironment,
    pub(crate) msbuild_verbosity_level: Option<VerbosityLevel>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum DotnetBuildpackConfigurationError {
    ExecutionEnvironmentError(ExecutionEnvironmentError),
    InvalidMsbuildVerbosityLevel(String),
}

impl TryFrom<&libcnb::Env> for DotnetBuildpackConfiguration {
    type Error = DotnetBuildpackConfigurationError;

    fn try_from(env: &libcnb::Env) -> Result<Self, Self::Error> {
        Ok(Self {
            build_configuration: env.get_string_lossy("BUILD_CONFIGURATION"),
            execution_environment: env
                .get_string_lossy("CNB_EXEC_ENV")
                .as_deref()
                .map_or_else(
                    || Ok(ExecutionEnvironment::Production),
                    ExecutionEnvironment::from_str,
                )
                .map_err(DotnetBuildpackConfigurationError::ExecutionEnvironmentError)?,
            msbuild_verbosity_level: detect_msbuild_verbosity_level(env).transpose()?,
        })
    }
}

fn detect_msbuild_verbosity_level(
    env: &libcnb::Env,
) -> Option<Result<VerbosityLevel, DotnetBuildpackConfigurationError>> {
    env.get("MSBUILD_VERBOSITY_LEVEL")
        .map(|value| value.to_string_lossy())
        .map(|value| value.parse())
}

impl FromStr for VerbosityLevel {
    type Err = DotnetBuildpackConfigurationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "q" | "quiet" => Ok(VerbosityLevel::Quiet),
            "m" | "minimal" => Ok(VerbosityLevel::Minimal),
            "n" | "normal" => Ok(VerbosityLevel::Normal),
            "d" | "detailed" => Ok(VerbosityLevel::Detailed),
            "diag" | "diagnostic" => Ok(VerbosityLevel::Diagnostic),
            _ => Err(
                DotnetBuildpackConfigurationError::InvalidMsbuildVerbosityLevel(value.to_string()),
            ),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ExecutionEnvironment {
    Production,
    Test,
}

impl FromStr for ExecutionEnvironment {
    type Err = ExecutionEnvironmentError;

    fn from_str(cnb_exec_env: &str) -> Result<Self, Self::Err> {
        match cnb_exec_env {
            "production" => Ok(ExecutionEnvironment::Production),
            "test" => Ok(ExecutionEnvironment::Test),
            _ => Err(ExecutionEnvironmentError::UnsupportedExecutionEnvironment(
                cnb_exec_env.to_string(),
            )),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ExecutionEnvironmentError {
    UnsupportedExecutionEnvironment(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum VerbosityLevel {
    Quiet,
    Minimal,
    Normal,
    Detailed,
    Diagnostic,
}

impl fmt::Display for VerbosityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerbosityLevel::Quiet => write!(f, "quiet"),
            VerbosityLevel::Minimal => write!(f, "minimal"),
            VerbosityLevel::Normal => write!(f, "normal"),
            VerbosityLevel::Detailed => write!(f, "detailed"),
            VerbosityLevel::Diagnostic => write!(f, "diagnostic"),
        }
    }
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
    fn test_buildpack_configuration_test_execution_environment() {
        let env = create_env(&[("CNB_EXEC_ENV", "test")]);
        let result = DotnetBuildpackConfiguration::try_from(&env).unwrap();

        assert_eq!(result.execution_environment, ExecutionEnvironment::Test);
    }

    #[test]
    fn test_parse_execution_environment() {
        let cases = [
            ("production", Ok(ExecutionEnvironment::Production)),
            ("test", Ok(ExecutionEnvironment::Test)),
            (
                "foo",
                Err(ExecutionEnvironmentError::UnsupportedExecutionEnvironment(
                    "foo".to_string(),
                )),
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(ExecutionEnvironment::from_str(input), expected);
        }
    }

    #[test]
    fn test_detect_msbuild_verbosity_level() {
        let valid_cases = [
            ("q", VerbosityLevel::Quiet),
            ("quiet", VerbosityLevel::Quiet),
            ("minimal", VerbosityLevel::Minimal),
            ("m", VerbosityLevel::Minimal),
            ("normal", VerbosityLevel::Normal),
            ("n", VerbosityLevel::Normal),
            ("detailed", VerbosityLevel::Detailed),
            ("d", VerbosityLevel::Detailed),
            ("diagnostic", VerbosityLevel::Diagnostic),
            ("diag", VerbosityLevel::Diagnostic),
        ];

        for (input, expected) in valid_cases {
            let result = input.parse();
            assert_eq!(result, Ok(expected));
        }

        let result = "invalid".parse::<VerbosityLevel>();
        assert!(matches!(
            result,
            Err(DotnetBuildpackConfigurationError::InvalidMsbuildVerbosityLevel(s)) if s == "invalid"
        ));
        assert!(detect_msbuild_verbosity_level(&Env::new()).is_none());
    }

    #[test]
    fn test_verbosity_level_display() {
        let cases = [
            (VerbosityLevel::Quiet, "quiet"),
            (VerbosityLevel::Minimal, "minimal"),
            (VerbosityLevel::Normal, "normal"),
            (VerbosityLevel::Detailed, "detailed"),
            (VerbosityLevel::Diagnostic, "diagnostic"),
        ];

        for (level, expected) in cases {
            assert_eq!(level.to_string(), expected);
        }
    }
}
