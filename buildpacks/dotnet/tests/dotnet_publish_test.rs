use crate::tests::{default_build_config, get_dotnet_arch};
use indoc::{formatdoc, indoc};
use libcnb_test::{assert_contains, assert_empty, PackResult, TestRunner};
use regex::Regex;

#[test]
#[ignore = "integration test"]
fn test_dotnet_publish_multi_tfm_solution() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/solution_with_web_and_console_projects"),
        |context| {
            assert_empty!(context.pack_stderr);

            let rid = get_rid();
            assert_contains!(context.pack_stdout, "Detected version requirement: `^8.0`");
            assert_contains!(
                context.pack_stdout,
                &format! {"worker -> /tmp/build_artifacts/bin/worker/release_{rid}/worker.dll"}
            );
            assert_contains!(
                context.pack_stdout,
                "worker -> /workspace/worker/bin/publish/"
            );
            assert_contains!(
                context.pack_stdout,
                &format! {"web -> /tmp/build_artifacts/bin/web/release_{rid}/web.dll" }
            );
            assert_contains!(context.pack_stdout, "web -> /workspace/web/bin/publish/");
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_publish_with_compilation_error() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/console_with_compilation_error")
            .expected_pack_result(PackResult::Failure),
        |context| {
            assert_contains!(
                &context.pack_stderr,
                &indoc! {r"
                  ! Unable to publish
                  !
                  ! The `dotnet publish` command exited unsuccessfully (exit status: 1).
                  !
                  ! This error usually happens due to compilation errors. Use the command output
                  ! above to troubleshoot and retry your build.
                  !
                  ! The publish process can also fail for a number of other reasons, such as
                  ! intermittent network issues, unavailability of the NuGet package feed and/or
                  ! other external dependencies, etc.
                  !
                  ! Try again to see if the error resolves itself."}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_publish_with_debug_configuration() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/basic_web_8.0_with_global_json")
            .env("BUILD_CONFIGURATION", "Debug"),
        |context| {
            assert_empty!(context.pack_stderr);

            let rid = get_rid();
            assert_contains!(
                &context.pack_stdout,
                &formatdoc! {r#"
                    - Running `dotnet publish /workspace/foo.csproj --runtime {rid} "-p:PublishDir=bin/publish" --artifacts-path /tmp/build_artifacts --configuration Debug`"#}
            );
            assert_contains!(
                replace_msbuild_log_patterns_with_placeholder(
                    &context.pack_stdout,
                    "<PLACEHOLDER>"
                ),
                &formatdoc! {r"
                  MSBuild version 17.8.3+195e7f5a3 for .NET
                          Determining projects to restore...
                          Restored /workspace/foo.csproj <PLACEHOLDER>.
                          foo -> /tmp/build_artifacts/bin/foo/debug_{rid}/foo.dll
                          foo -> /workspace/bin/publish/"}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_publish_process_registration_with_procfile() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/basic_web_9.0_with_procfile"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                &context.pack_stdout,
                indoc! { r"
                    - Process types
                      - Detecting process types from published artifacts
                      - Found `foo`: bash -c cd bin/publish; ./foo --urls http://*:$PORT
                      - Procfile detected
                      - Skipping process type registration (add process types to your Procfile as needed)"}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_publish_process_registration_without_procfile() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/basic_web_9.0"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                &context.pack_stdout,
                indoc! { r"
                - Process types
                  - Detecting process types from published artifacts
                  - Found `foo`: bash -c cd bin/publish; ./foo --urls http://*:$PORT
                  - No Procfile detected
                  - Registering detected process types as launch processes"}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_publish_process_registration_without_process_types() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/class_library"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                &context.pack_stdout,
                indoc! { r"
                - Process types
                  - Detecting process types from published artifacts
                  - No processes were detected"}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_publish_with_global_json_and_custom_verbosity_level() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/basic_web_8.0_with_global_json")
          .env("MSBUILD_VERBOSITY_LEVEL", "normal"),
        |context| {
            assert_empty!(context.pack_stderr);
            let rid = get_rid();

            assert_contains!(
              replace_msbuild_log_patterns_with_placeholder(&context.pack_stdout, "<PLACEHOLDER>"), 
              &formatdoc! {r#"
                - Publish app
                  - Running `dotnet publish /workspace/foo.csproj --runtime {rid} "-p:PublishDir=bin/publish" --artifacts-path /tmp/build_artifacts --verbosity normal`
                
                      MSBuild version 17.8.3+195e7f5a3 for .NET
                      Build started <PLACEHOLDER>.
                           1>Project "/workspace/foo.csproj" on node 1 (Restore target(s)).
                           1>_GetAllRestoreProjectPathItems:
                               Determining projects to restore...
                             Restore:
                               X.509 certificate chain validation will use the fallback certificate bundle at '/layers/heroku_dotnet/sdk/sdk/8.0.101/trustedroots/codesignctl.pem'.
                               X.509 certificate chain validation will use the fallback certificate bundle at '/layers/heroku_dotnet/sdk/sdk/8.0.101/trustedroots/timestampctl.pem'.
                               Restoring packages for /workspace/foo.csproj..."#}
            );

            assert_contains!(
              replace_msbuild_log_patterns_with_placeholder(&context.pack_stdout, "<PLACEHOLDER>"), 
              "Time Elapsed <PLACEHOLDER>"
            );
        },
    );
}

fn get_rid() -> String {
    format!("linux-{}", get_dotnet_arch())
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
