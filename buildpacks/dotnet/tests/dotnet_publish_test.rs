use crate::tests::{default_build_config, get_dotnet_arch};
use indoc::{formatdoc, indoc};
use libcnb_test::{ContainerConfig, PackResult, TestRunner, assert_contains, assert_empty};
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
                &context.pack_stdout,
                &indoc! {r"
                  ! Unable to publish
                  !
                  ! The `dotnet publish` command failed (exit status: 1).
                  !
                  ! The most common cause is a compilation error. Review the command output above
                  ! to find and fix the issue.
                  !
                  ! The failure may also be temporary due to a network or service outage. Retrying
                  ! your build often resolves this.
                  !
                  ! If the log suggests a NuGet issue, check the service status before retrying:
                  ! https://status.nuget.org"}
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
fn test_dotnet_publish_with_project_toml_configuration() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/project_with_project_toml"),
        |context| {
            let rid = get_rid();
            assert_contains!(
                &context.pack_stdout,
                &formatdoc! {r#"
                    - Running `dotnet publish /workspace/foo.csproj --runtime {rid} "-p:PublishDir=bin/publish" --artifacts-path /tmp/build_artifacts --configuration Debug --verbosity quiet`"#}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_solution_detection_with_multiple_workspace_root_solutions() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/multiple_solutions")
            .expected_pack_result(PackResult::Failure),
        |context| {
            assert_contains!(context.pack_stdout, "! Multiple .NET solution files");
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_solution_detection_with_multiple_workspace_root_solutions_and_project_toml_solution_file() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/multiple_solutions_with_project_toml"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                "- Using configured solution file: `foo.sln`"
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_solution_detection_with_multiple_workspace_root_solutions_and_solution_file_env_var() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/multiple_solutions").env("SOLUTION_FILE", "bar.sln"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                "- Using configured solution file: `bar.sln`"
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_solution_file_env_var_takes_precedence_over_project_toml() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/multiple_solutions_with_project_toml")
            .env("SOLUTION_FILE", "baz.slnx"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                "- Using configured solution file: `baz.slnx`"
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_publish_file_based_app_basic_console() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/file_based_app_basic_console"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                "- Detected .NET file-based app: `/workspace/foo.cs`"
            );
            assert_contains!(
                context.pack_stdout,
                "- Detected version requirement: `^10.0`"
            );
            assert_contains!(
                context.pack_stdout,
                "- Running `dotnet publish /workspace/foo.cs"
            );
            assert_contains!(context.pack_stdout, "foo -> /workspace/bin/publish/");
            assert_contains!(context.pack_stdout, "- Analyzing candidates:");
            assert_contains!(
                context.pack_stdout,
                "- `foo.cs`: Found executable at `bin/publish/foo`"
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_publish_file_based_app_basic_web() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/file_based_app_basic_web"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                "- Detected .NET file-based app: `/workspace/foo.cs`"
            );
            assert_contains!(
                context.pack_stdout,
                "- Detected version requirement: `^10.0`"
            );
            assert_contains!(
                context.pack_stdout,
                "- Running `dotnet publish /workspace/foo.cs"
            );
            assert_contains!(context.pack_stdout, "foo -> /workspace/bin/publish/");
            assert_contains!(context.pack_stdout, "- Analyzing candidates:");
            assert_contains!(
                context.pack_stdout,
                "- `foo.cs`: Found executable at `bin/publish/foo`"
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
                      - Analyzing candidates:
                      - `foo.csproj`: Found executable at `bin/publish/foo`
                      - Procfile detected
                      - Skipping automatic registration (Procfile takes precedence)
                      - Available process types (for reference):
                      - `web`: bash -c cd bin/publish; ./foo --urls http://*:$PORT"}
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
                  - Analyzing candidates:
                  - `foo.csproj`: Found executable at `bin/publish/foo`
                  - No Procfile detected
                  - Registering launch processes:
                  - `web`: bash -c cd bin/publish; ./foo --urls http://*:$PORT
                - Done"}
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
                  - No candidate projects detected"}
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

#[test]
#[ignore = "integration test"]
fn test_dotnet_publish_with_space_in_project_filename() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/solution_with_spaces"),
        |context| {
            assert_empty!(&context.pack_stderr);
            assert_contains!(
                &context.pack_stdout,
                r#"Running `dotnet publish "/workspace/solution with spaces.sln""#
            );

            assert_contains!(
                &context.pack_stdout,
                r"- `console app/console app.csproj`: Found executable at `console app/bin/publish/console app`"
            );
            assert_contains!(
                &context.pack_stdout,
                r"- `console-app`: bash -c cd 'console app/bin/publish'; ./'console app'"
            );

            context.start_container(
                ContainerConfig::new().entrypoint("console-app"),
                |container| {
                    let log_output = container.logs_wait();

                    assert_empty!(log_output.stderr);
                    assert_contains!(log_output.stdout, "Hello, World!");
                },
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_publish_with_updated_process_type_name_heroku_warning() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/solution_with_web_and_console_projects")
            .env("STACK", "heroku-24"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                &formatdoc! {r"
                  - Process types
                    - Detecting process types from published artifacts
                    - Analyzing candidates:
                    - `web/web.csproj`: Found executable at `web/bin/publish/web`
                    - `worker/worker.csproj`: Found executable at `worker/bin/publish/worker`
                    - No Procfile detected
                    - Registering launch processes:
                    - `web`: bash -c cd web/bin/publish; ./web --urls http://*:$PORT
                    - `worker`: bash -c cd worker/bin/publish; ./worker
                  - Done"}
            );
            assert_contains!(context.pack_stdout, "web -> /workspace/web/bin/publish/");
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_publish_slnx_with_web_and_console_projects() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/solution_slnx_with_web_and_console_projects")
            .env("STACK", "heroku-24"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                &formatdoc! {r"
                  - Process types
                    - Detecting process types from published artifacts
                    - Analyzing candidates:
                    - `web/web.csproj`: Found executable at `web/bin/publish/web`
                    - `worker/worker.csproj`: Found executable at `worker/bin/publish/worker`
                    - No Procfile detected
                    - Registering launch processes:
                    - `web`: bash -c cd web/bin/publish; ./web --urls http://*:$PORT
                    - `worker`: bash -c cd worker/bin/publish; ./worker
                  - Done"}
            );
            assert_contains!(context.pack_stdout, "web -> /workspace/web/bin/publish/");
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
