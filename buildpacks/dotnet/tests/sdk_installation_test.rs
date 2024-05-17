use crate::tests::default_build_config;
use indoc::indoc;
use libcnb_test::{assert_contains, assert_empty, TestRunner};

// These are simply a couple of test drafts (to test current behavior of the buildpack itself,
// as well as running integration tests locally/on GitHub).
// TODO: Update when logic to determine SDK version is implemented, and allow for multiple archs.
#[test]
#[ignore = "integration test"]
#[cfg(target_arch = "x86_64")]
fn sdk_installation_test() {
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
                    Resolved .NET SDK version 8.0.300 (linux-amd64)
                    Downloading .NET SDK version 8.0.300 from https://download.visualstudio.microsoft.com/download/pr/4a252cd9-d7b7-41bf-a7f0-b2b10b45c068/1aff08f401d0e3980ac29ccba44efb29/dotnet-sdk-8.0.300-linux-x64.tar.gz
                    Verifying checksum
                    Installing .NET SDK
                "#}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
#[cfg(target_arch = "aarch64")]
fn sdk_installation_test() {
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
                    Resolved .NET SDK version 8.0.300 (linux-arm64)
                    Downloading .NET SDK version 8.0.300 from https://download.visualstudio.microsoft.com/download/pr/54e5bb2e-bdd6-496d-8aba-4ed14658ee91/34fd7327eadad7611bded51dcda44c35/dotnet-sdk-8.0.300-linux-arm64.tar.gz
                    Verifying checksum
                    Installing .NET SDK
                "#}
            );
        },
    );
}
