use crate::dotnet_publish_command::VerbosityLevel;

pub(crate) struct DotnetBuildpackConfiguration {
    pub(crate) build_configuration: String,
    pub(crate) msbuild_verbosity_level: VerbosityLevel,
}

#[derive(Debug, PartialEq)]
pub(crate) enum DotnetBuildpackConfigurationError {
    InvalidMsbuildVerbosityLevel(String),
}

impl TryFrom<&libcnb::Env> for DotnetBuildpackConfiguration {
    type Error = DotnetBuildpackConfigurationError;

    fn try_from(env: &libcnb::Env) -> Result<Self, Self::Error> {
        Ok(Self {
            build_configuration: String::from("Release"),
            msbuild_verbosity_level: detect_msbuild_verbosity_level(env)?,
        })
    }
}

fn detect_msbuild_verbosity_level(
    env: &libcnb::Env,
) -> Result<VerbosityLevel, DotnetBuildpackConfigurationError> {
    env.get("MSBUILD_VERBOSITY_LEVEL")
        .map(|value| value.to_string_lossy())
        .map_or(Ok(VerbosityLevel::Minimal), |value| {
            match value.to_lowercase().as_str() {
                "q" | "quiet" => Ok(VerbosityLevel::Quiet),
                "m" | "minimal" => Ok(VerbosityLevel::Minimal),
                "n" | "normal" => Ok(VerbosityLevel::Normal),
                "d" | "detailed" => Ok(VerbosityLevel::Detailed),
                "diag" | "diagnostic" => Ok(VerbosityLevel::Diagnostic),
                _ => Err(
                    DotnetBuildpackConfigurationError::InvalidMsbuildVerbosityLevel(
                        value.to_string(),
                    ),
                ),
            }
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
    fn test_detect_msbuild_verbosity_level() {
        let cases = [
            (Some("quiet"), Ok(VerbosityLevel::Quiet)),
            (Some("q"), Ok(VerbosityLevel::Quiet)),
            (Some("minimal"), Ok(VerbosityLevel::Minimal)),
            (Some("m"), Ok(VerbosityLevel::Minimal)),
            (Some("normal"), Ok(VerbosityLevel::Normal)),
            (Some("n"), Ok(VerbosityLevel::Normal)),
            (Some("detailed"), Ok(VerbosityLevel::Detailed)),
            (Some("d"), Ok(VerbosityLevel::Detailed)),
            (Some("diagnostic"), Ok(VerbosityLevel::Diagnostic)),
            (Some("diag"), Ok(VerbosityLevel::Diagnostic)),
            (
                Some("invalid"),
                Err(
                    DotnetBuildpackConfigurationError::InvalidMsbuildVerbosityLevel(
                        "invalid".to_string(),
                    ),
                ),
            ),
            (None, Ok(VerbosityLevel::Minimal)),
        ];

        for (input, expected) in &cases {
            let env = match input {
                Some(value) => create_env(&[("MSBUILD_VERBOSITY_LEVEL", value)]),
                None => Env::new(),
            };
            let result = detect_msbuild_verbosity_level(&env);
            assert_eq!(result, *expected);
        }
    }
}
