use crate::tests::{default_build_config, get_dotnet_arch};
use indoc::{formatdoc, indoc};
use libcnb_test::{assert_contains, assert_empty, BuildpackReference, ContainerConfig, TestRunner};

#[test]
#[ignore = "integration test"]
fn test_sdk_resolution_with_target_framework_8_0() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/basic_web_8.0"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                &indoc! {r"
                    - SDK version detection
                      - Detected .NET project: `/workspace/foo.csproj`
                      - Inferring version requirement from `/workspace/foo.csproj`
                      - Detected version requirement: `^8.0`
                      - Resolved .NET SDK version `8.0"}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_sdk_resolution_with_target_framework_9_0() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/basic_web_9.0"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                &indoc! {r"
                    - SDK version detection
                      - Detected .NET project: `/workspace/foo.csproj`
                      - Inferring version requirement from `/workspace/foo.csproj`
                      - Detected version requirement: `^9.0`
                      - Resolved .NET SDK version `9.0"}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_sdk_resolution_with_solution_file() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/solution_with_web_and_console_projects"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                &indoc! {r"
                    - SDK version detection
                      - Detected .NET solution: `/workspace/foo.sln`
                      - Inferring version requirement from `/workspace/foo.sln`
                      - Detected version requirement: `^8.0"}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_sdk_basic_install_build_environment() {
    let mut config = default_build_config("tests/fixtures/console_with_nuget_package");
    config.buildpacks(vec![
        BuildpackReference::CurrentCrate,
        BuildpackReference::Other("file://tests/fixtures/testing_buildpack".to_string()),
    ]);

    TestRunner::default().build(&config, |context| {
        assert_empty!(context.pack_stderr);
        assert_contains!(
            context.pack_stdout,
            &indoc! {"
                ## Testing buildpack ##
                DOTNET_CLI_HOME=/layers/heroku_dotnet/dotnet-cli
                DOTNET_CLI_TELEMETRY_OPTOUT=true
                DOTNET_EnableWriteXorExecute=0
                DOTNET_NOLOGO=true
                DOTNET_ROOT=/layers/heroku_dotnet/sdk
                DOTNET_RUNNING_IN_CONTAINER=true
                NUGET_PACKAGES=/layers/heroku_dotnet/nuget-cache
                NUGET_XMLDOC_MODE=skip
                PATH=/layers/heroku_dotnet/sdk:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"}
        );
    });
}

#[test]
#[ignore = "integration test"]
fn test_sdk_basic_install_test_execution_environment() {
    let mut config = default_build_config("tests/fixtures/project_with_nuget_sdk_and_global_json");
    config.env("CNB_EXEC_ENV", "test");

    TestRunner::default().build(&config, |context| {
        assert_empty!(context.pack_stderr);

        context.start_container(ContainerConfig::new().entrypoint("test"), |container| {
            let log_output = container.logs_wait();
            let dotnet_arch = get_dotnet_arch();

            assert_empty!(log_output.stderr);
            assert_contains!(
                log_output.stdout,
                &formatdoc! {"
                    foo -> /workspace/bin/Debug/net9.0/foo.dll
                      Run tests: '/workspace/bin/Debug/net9.0/foo.dll' [net9.0|{dotnet_arch}]
                      Passed! - Failed: 0, Passed: 1, Skipped: 0, Total: 1"}
            );
            assert_contains!(
                log_output.stdout,
                &format!(
                    "Tests succeeded: '/workspace/bin/Debug/net9.0/foo.dll' [net9.0|{dotnet_arch}"
                )
            );
        });
    });
}

#[cfg(target_arch = "x86_64")]
#[test]
#[ignore = "integration test"]
fn test_sdk_installation_with_global_json() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/basic_web_8.0_with_global_json"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                &indoc! {r"
                    - SDK version detection
                      - Detected .NET project: `/workspace/foo.csproj`
                      - Detecting version requirement from root global.json file
                      - Detected version requirement: `=8.0.101`
                      - Resolved .NET SDK version `8.0.101` (linux-amd64)
                    - SDK installation
                      - Downloading SDK from https://download.visualstudio.microsoft.com/download/pr/9454f7dc-b98e-4a64-a96d-4eb08c7b6e66/da76f9c6bc4276332b587b771243ae34/dotnet-sdk-8.0.101-linux-x64.tar.gz"
                }
            );
            assert_contains!(
                context.pack_stdout,
                indoc! {r"
                    - Verifying SDK checksum
                      - Installing SDK"}
            );
            // Verify SDK caching behavior
            let config = context.config.clone();
            context.rebuild(config, |ctx| {
                assert_contains!(ctx.pack_stdout, "Reusing cached SDK (version 8.0.101)");
            });
        },
    );
}

#[cfg(target_arch = "aarch64")]
#[test]
#[ignore = "integration test"]
fn test_sdk_installation_with_global_json() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/basic_web_8.0_with_global_json"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                &indoc! {r"
                    - SDK version detection
                      - Detected .NET project: `/workspace/foo.csproj`
                      - Detecting version requirement from root global.json file
                      - Detected version requirement: `=8.0.101`
                      - Resolved .NET SDK version `8.0.101` (linux-arm64)
                    - SDK installation
                      - Downloading SDK from https://download.visualstudio.microsoft.com/download/pr/092bec24-9cad-421d-9b43-458b3a7549aa/84280dbd1eef750f9ed1625339235c22/dotnet-sdk-8.0.101-linux-arm64.tar.gz"
                }
            );
            assert_contains!(
                context.pack_stdout,
                indoc! {r"
                    - Verifying SDK checksum
                      - Installing SDK"}
            );
            // Verify SDK caching behavior
            let config = context.config.clone();
            context.rebuild(config, |ctx| {
                assert_contains!(ctx.pack_stdout, "Reusing cached SDK (version 8.0.101)");
            });
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_sdk_installation_with_global_json_prerelease_sdk() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/basic_web_9.0_with_global_json_prerelease_sdk"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                &indoc! {r"
                    - SDK version detection
                      - Detected .NET project: `/workspace/foo.csproj`
                      - Detecting version requirement from root global.json file
                      - Detected version requirement: `=9.0.100-rc.1.24452.12`
                      - Resolved .NET SDK version `9.0.100-rc.1.24452.12`"
                }
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_sdk_installation_with_global_json_project_sdk_version_config() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/project_with_nuget_sdk_and_global_json"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                "- Inferring version requirement from `/workspace/foo.csproj`"
            );
        },
    );
}
