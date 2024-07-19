use crate::dotnet_publish_command::VerbosityLevel;

pub(crate) struct DotnetBuildpackConfiguration {
    pub(crate) msbuild_verbosity_level: VerbosityLevel,
}

#[derive(Debug)]
pub(crate) enum DotnetBuildpackConfigurationError {
    InvalidMsbuildVerbosityLevel(String),
}

impl TryFrom<&libcnb::Env> for DotnetBuildpackConfiguration {
    type Error = DotnetBuildpackConfigurationError;

    fn try_from(env: &libcnb::Env) -> Result<Self, Self::Error> {
        Ok(Self {
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
                "diag" | "diagnostics" => Ok(VerbosityLevel::Diagnostic),
                _ => Err(
                    DotnetBuildpackConfigurationError::InvalidMsbuildVerbosityLevel(
                        value.to_string(),
                    ),
                ),
            }
        })
}
