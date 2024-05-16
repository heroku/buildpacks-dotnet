use libcnb_test::BuildConfig;
use std::path::Path;

mod detect_test;

pub(crate) fn default_build_config(fixture_path: impl AsRef<Path>) -> BuildConfig {
    #[cfg(target_arch = "amd64")]
    let target_triple = "x86_64-unknown-linux-musl";
    #[cfg(target_arch = "aarch64")]
    let target_triple = "aarch64-unknown-linux-musl";

    let mut config = BuildConfig::new("heroku/builder:24", fixture_path);
    config.target_triple(target_triple);
    config
}
