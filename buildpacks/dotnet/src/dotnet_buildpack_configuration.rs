use std::fmt;

pub(crate) struct DotnetBuildpackConfiguration {
    pub(crate) build_configuration: Option<String>,
    pub(crate) msbuild_verbosity_level: Option<VerbosityLevel>,
    pub(crate) sdk_command: Option<String>,
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
            msbuild_verbosity_level: detect_msbuild_verbosity_level(env).transpose()?,
            sdk_command: env
                .get("DOTNET_SDK_COMMAND")
                .map(|value| value.to_string_lossy().to_string()),
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
