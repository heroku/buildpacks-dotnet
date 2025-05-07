use crate::tests::default_build_config;
use crate::tests::get_dotnet_arch;
use indoc::formatdoc;
use indoc::indoc;
use libcnb_test::{BuildpackReference, TestRunner, assert_contains, assert_empty};

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
fn test_sdk_installation_with_global_json() {
    let dotnet_arch = get_dotnet_arch();
    let artifact_arch = match dotnet_arch.as_str() {
        "x64" => "amd64",
        "arm64" => "arm64",
        _ => panic!("Unsupported architecture for this test: {dotnet_arch}"),
    };
    TestRunner::default().build(
        default_build_config("tests/fixtures/basic_web_8.0_with_global_json"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                &formatdoc!("
                    - SDK version detection
                      - Detected .NET project: `/workspace/foo.csproj`
                      - Detecting version requirement from root global.json file
                      - Detected version requirement: `=8.0.101`
                      - Resolved .NET SDK version `8.0.101` (linux-{artifact_arch})
                    - SDK installation
                      - Downloading SDK from https://builds.dotnet.microsoft.com/dotnet/Sdk/8.0.101/dotnet-sdk-8.0.101-linux-{dotnet_arch}.tar.gz"
                )
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
