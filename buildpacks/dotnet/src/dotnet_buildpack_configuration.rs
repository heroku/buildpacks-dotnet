use crate::project_toml::DotnetConfig;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub(crate) struct DotnetBuildpackConfiguration {
    pub(crate) build_configuration: Option<String>,
    pub(crate) execution_environment: ExecutionEnvironment,
    pub(crate) msbuild_verbosity_level: Option<VerbosityLevel>,
    pub(crate) solution_file: Option<PathBuf>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum DotnetBuildpackConfigurationError {
    ExecutionEnvironment(ExecutionEnvironmentError),
    VerbosityLevel(ParseVerbosityLevelError),
    InvalidSolutionFile(String),
}

impl DotnetBuildpackConfiguration {
    pub(crate) fn try_from_env_and_project_toml(
        env: &libcnb::Env,
        project_toml_config: Option<&DotnetConfig>,
    ) -> Result<Self, DotnetBuildpackConfigurationError> {
        let msbuild_config = project_toml_config.and_then(|config| config.msbuild.as_ref());

        let solution_file = env
            .get_string_lossy("SOLUTION_FILE")
            .map(PathBuf::from)
            .or_else(|| project_toml_config.and_then(|config| config.solution_file.clone()));

        if let Some(ref path) = solution_file {
            let extension = path.extension().and_then(|ext| ext.to_str());
            if !matches!(extension, Some("sln" | "slnx")) {
                return Err(DotnetBuildpackConfigurationError::InvalidSolutionFile(
                    path.to_string_lossy().to_string(),
                ));
            }
        }

        Ok(Self {
            build_configuration: env
                .get_string_lossy("BUILD_CONFIGURATION")
                .or_else(|| msbuild_config?.configuration.clone()),
            execution_environment: env
                .get_string_lossy("CNB_EXEC_ENV")
                .as_deref()
                .map_or_else(
                    || Ok(ExecutionEnvironment::Production),
                    ExecutionEnvironment::from_str,
                )
                .map_err(DotnetBuildpackConfigurationError::ExecutionEnvironment)?,
            msbuild_verbosity_level: env
                .get_string_lossy("MSBUILD_VERBOSITY_LEVEL")
                .as_deref()
                .or_else(|| msbuild_config?.verbosity.as_deref())
                .map(str::parse)
                .transpose()
                .map_err(DotnetBuildpackConfigurationError::VerbosityLevel)?,
            solution_file,
        })
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ExecutionEnvironment {
    Production,
    Test,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ExecutionEnvironmentError {
    UnsupportedExecutionEnvironment(String),
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum VerbosityLevel {
    Quiet,
    Minimal,
    Normal,
    Detailed,
    Diagnostic,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ParseVerbosityLevelError(pub(crate) String);

impl FromStr for VerbosityLevel {
    type Err = ParseVerbosityLevelError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "q" | "quiet" => Ok(VerbosityLevel::Quiet),
            "m" | "minimal" => Ok(VerbosityLevel::Minimal),
            "n" | "normal" => Ok(VerbosityLevel::Normal),
            "d" | "detailed" => Ok(VerbosityLevel::Detailed),
            "diag" | "diagnostic" => Ok(VerbosityLevel::Diagnostic),
            _ => Err(ParseVerbosityLevelError(value.to_string())),
        }
    }
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
    use crate::project_toml::MsbuildConfig;
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
        let result =
            DotnetBuildpackConfiguration::try_from_env_and_project_toml(&env, None).unwrap();

        assert_eq!(
            result,
            DotnetBuildpackConfiguration {
                build_configuration: None,
                execution_environment: ExecutionEnvironment::Production,
                msbuild_verbosity_level: None,
                solution_file: None
            }
        );
    }

    #[test]
    fn test_project_toml_overrides_default_config() {
        let project_toml_config = DotnetConfig {
            msbuild: Some(MsbuildConfig {
                configuration: Some("Debug".to_string()),
                verbosity: Some("Detailed".to_string()),
            }),
            solution_file: Some(PathBuf::from("foo.sln")),
        };
        let result = DotnetBuildpackConfiguration::try_from_env_and_project_toml(
            &create_env(&[]),
            Some(&project_toml_config),
        )
        .unwrap();

        assert_eq!(result.solution_file, Some(PathBuf::from("foo.sln")));
        assert_eq!(result.build_configuration, Some("Debug".to_string()));
        assert_eq!(
            result.msbuild_verbosity_level,
            Some(VerbosityLevel::Detailed)
        );
    }

    #[test]
    fn test_env_overrides_project_toml() {
        let env = create_env(&[
            ("BUILD_CONFIGURATION", "Release"),
            ("MSBUILD_VERBOSITY_LEVEL", "Detailed"),
            ("SOLUTION_FILE", "env-solution.sln"),
        ]);
        let project_toml_config = DotnetConfig {
            msbuild: Some(MsbuildConfig {
                configuration: Some("Debug".to_string()),
                verbosity: Some("Quiet".to_string()),
            }),
            solution_file: Some(PathBuf::from("toml-solution.sln")),
        };
        let result = DotnetBuildpackConfiguration::try_from_env_and_project_toml(
            &env,
            Some(&project_toml_config),
        )
        .unwrap();

        assert_eq!(result.build_configuration, Some("Release".to_string()));
        assert_eq!(
            result.msbuild_verbosity_level,
            Some(VerbosityLevel::Detailed)
        );
        assert_eq!(
            result.solution_file,
            Some(PathBuf::from("env-solution.sln"))
        );
    }

    #[test]
    fn test_env_vars_override_default_config() {
        let env = create_env(&[
            ("BUILD_CONFIGURATION", "Release"),
            ("MSBUILD_VERBOSITY_LEVEL", "Detailed"),
            ("CNB_EXEC_ENV", "test"),
            ("SOLUTION_FILE", "env-solution.sln"),
        ]);
        let result =
            DotnetBuildpackConfiguration::try_from_env_and_project_toml(&env, None).unwrap();

        assert_eq!(result.build_configuration, Some("Release".to_string()));
        assert_eq!(result.execution_environment, ExecutionEnvironment::Test);
        assert_eq!(
            result.msbuild_verbosity_level,
            Some(VerbosityLevel::Detailed)
        );
        assert_eq!(
            result.solution_file,
            Some(PathBuf::from("env-solution.sln"))
        );
    }

    #[test]
    fn test_parse_execution_environment() {
        assert_eq!("production".parse(), Ok(ExecutionEnvironment::Production));
        assert_eq!("test".parse(), Ok(ExecutionEnvironment::Test));
        assert_eq!(
            "invalid".parse::<ExecutionEnvironment>(),
            Err(ExecutionEnvironmentError::UnsupportedExecutionEnvironment(
                "invalid".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_msbuild_verbosity_level() {
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
            Err(ParseVerbosityLevelError(s)) if s == "invalid"
        ));
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
