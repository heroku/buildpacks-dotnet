use libcnb_test::BuildConfig;
use std::env;
use std::path::Path;

mod detect_test;
mod dotnet_publish_test;
mod dotnet_restore_tools_test;
mod nuget_layer_test;
mod runtime_dependencies_test;
mod sdk_installation_test;
mod test_execution_environment_test;

const DEFAULT_BUILDER: &str = "heroku/builder:24";

pub(crate) fn default_build_config(fixture_path: impl AsRef<Path>) -> BuildConfig {
    let builder = builder();
    let mut config = BuildConfig::new(&builder, fixture_path);

    // TODO: Once Pack build supports `--platform` and libcnb-test adjusted accordingly, change this
    // to allow configuring the target arch independently of the builder name (eg via env var).
    let target_triple = match builder.as_str() {
        // Compile the buildpack for ARM64 if the builder supports multi-arch and the host is ARM64.
        "heroku/builder:24" if cfg!(target_arch = "aarch64") => "aarch64-unknown-linux-musl",
        _ => "x86_64-unknown-linux-musl",
    };
    config.target_triple(target_triple);
    config
}

fn get_dotnet_arch() -> String {
    #[cfg(target_arch = "x86_64")]
    let arch = "x64";
    #[cfg(target_arch = "aarch64")]
    let arch = "arm64";

    arch.to_string()
}

fn builder() -> String {
    env::var("INTEGRATION_TEST_BUILDER").unwrap_or(DEFAULT_BUILDER.to_string())
}
