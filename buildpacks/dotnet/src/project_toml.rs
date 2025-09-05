use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct ProjectToml {
    com: Option<ComSection>,
}

#[derive(Debug, Deserialize)]
struct ComSection {
    heroku: Option<HerokuSection>,
}

#[derive(Debug, Deserialize)]
struct HerokuSection {
    buildpacks: Option<BuildpacksSection>,
}

#[derive(Debug, Deserialize)]
struct BuildpacksSection {
    dotnet: Option<DotnetConfig>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DotnetConfig {
    pub(crate) msbuild: Option<MsbuildConfig>,
    pub(crate) solution_file: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MsbuildConfig {
    pub(crate) configuration: Option<String>,
    pub(crate) verbosity: Option<String>,
}

pub(crate) fn parse(contents: &str) -> Result<Option<DotnetConfig>, toml::de::Error> {
    toml::from_str::<ProjectToml>(contents).map(|project_toml| {
        project_toml
            .com
            .and_then(|c| c.heroku)
            .and_then(|h| h.buildpacks)
            .and_then(|b| b.dotnet)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let project_toml_content = r#"
[com.heroku.buildpacks.dotnet]
solution_file = "foo.sln"
msbuild.configuration = "Debug"
msbuild.verbosity = "Detailed"
"#;

        let result = parse(project_toml_content).unwrap();

        assert!(result.is_some());

        let config = result.unwrap();
        assert_eq!(config.solution_file, Some(PathBuf::from("foo.sln")));
        assert_eq!(
            config.msbuild.as_ref().unwrap().configuration,
            Some("Debug".to_string())
        );
        assert_eq!(
            config.msbuild.as_ref().unwrap().verbosity,
            Some("Detailed".to_string())
        );
    }

    #[test]
    fn test_parse_missing_dotnet_section() {
        let project_toml_content = r#"
[com.heroku.buildpacks.other]
some.setting = "value"
"#;

        let result = parse(project_toml_content).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_toml() {
        let project_toml_content = r#"
[com.heroku.buildpacks.dotnet
msbuild.configuration = "Debug"
"#;

        let result = parse(project_toml_content);

        assert!(result.is_err());
        assert!(matches!(result, Err(toml::de::Error { .. })));
    }
}
