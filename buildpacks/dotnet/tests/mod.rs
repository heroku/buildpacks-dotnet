use libcnb_test::BuildConfig;
use regex::Regex;
use std::path::Path;

mod detect_test;
mod dotnet_publish_test;
mod dotnet_test_test;
mod nuget_layer_test;
mod sdk_installation_test;

pub(crate) fn default_build_config(fixture_path: impl AsRef<Path>) -> BuildConfig {
    #[cfg(target_arch = "x86_64")]
    let target_triple = "x86_64-unknown-linux-musl";
    #[cfg(target_arch = "aarch64")]
    let target_triple = "aarch64-unknown-linux-musl";

    let mut config = BuildConfig::new("heroku/builder:24", fixture_path);
    config.target_triple(target_triple);
    config
}

fn get_rid() -> String {
    format!("linux-{}", get_dotnet_arch())
}

fn get_dotnet_arch() -> String {
    #[cfg(target_arch = "x86_64")]
    let arch = "x64";
    #[cfg(target_arch = "aarch64")]
    let arch = "arm64";

    arch.to_string()
}

fn replace_msbuild_log_patterns_with_placeholder(input: &str, placeholder: &str) -> String {
    // Define regex patterns for dynamic/undeterministic msbuild log output to replace for simple integration test assertions.
    let patterns = vec![
        // Date-time pattern
        r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}",
        // Elapsed time pattern
        r"\d{2}:\d{2}:\d{2}\.\d{2}",
        // Server message with UUID pattern
        r"server - server processed compilation - [0-9a-fA-F-]{36}",
        // Parentheses text pattern (contains elapsed time in various forms)
        r"\(in [^)]+\)",
        // Milliseconds pattern
        r"\b\d+ms\b",
        // Section between _CopyOutOfDateSourceItemsToOutputDirectory and _CopyResolvedFilesToPublishAlways pattern:
        // (Log entries in these sections are not written deterministically).
        r"(?s)_CopyOutOfDateSourceItemsToOutputDirectory:.*?_CopyResolvedFilesToPublishAlways:",
    ];

    let mut result = input.to_string();
    for pattern in patterns {
        let regex = Regex::new(pattern).unwrap();
        result = regex.replace_all(&result, placeholder).to_string();
    }

    result
}
