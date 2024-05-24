use crate::tests::default_build_config;
use indoc::indoc;
use libcnb_test::{assert_contains, assert_empty, TestRunner};

#[test]
#[ignore = "integration test"]
fn test_sdk_resolution_with_target_framework() {
    TestRunner::default().build(
        default_build_config( "tests/fixtures/basic_web_8.0"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                &indoc! {r#"
                    [.NET SDK]
                    Detected .NET project file: /workspace/foo.csproj
                    Project type is WebApplication using SDK "Microsoft.NET.Sdk.Web" specifies TFM "net8.0"
                    Inferred SDK version requirement: ^8.0
                    Resolved .NET SDK version 8.0."#}
            );
        },
    );
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
                    [.NET SDK]
                    Detected .NET project file: /workspace/foo.csproj
                    Detected global.json file in the root directory
                    Inferred SDK version requirement: =8.0.101
                    Resolved .NET SDK version 8.0.101 (linux-amd64)
                    Downloading .NET SDK version 8.0.101 from https://download.visualstudio.microsoft.com/download/pr/9454f7dc-b98e-4a64-a96d-4eb08c7b6e66/da76f9c6bc4276332b587b771243ae34/dotnet-sdk-8.0.101-linux-x64.tar.gz
                    Verifying checksum
                    Installing .NET SDK"
                }
            );
            // Verify SDK caching behavior
            let config = context.config.clone();
            context.rebuild(config, |ctx| {
                assert_contains!(ctx.pack_stdout, "Reusing cached .NET SDK version: 8.0.101");
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
                    [.NET SDK]
                    Detected .NET project file: /workspace/foo.csproj
                    Detected global.json file in the root directory
                    Inferred SDK version requirement: =8.0.101
                    Resolved .NET SDK version 8.0.101 (linux-arm64)
                    Downloading .NET SDK version 8.0.101 from https://download.visualstudio.microsoft.com/download/pr/092bec24-9cad-421d-9b43-458b3a7549aa/84280dbd1eef750f9ed1625339235c22/dotnet-sdk-8.0.101-linux-arm64.tar.gz
                    Verifying checksum
                    Installing .NET SDK"
                }
            );
        },
    );
}
