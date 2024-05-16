use crate::tests::default_build_config;
use indoc::formatdoc;
use libcnb_test::{assert_contains, assert_empty, TestRunner};

// This is just a test stub to test current behavior (of the buildpack itself, and running integration tests locally/on GitHub).
// TODO: Update when logic to determine SDK version is implemented, allow for multiple archs.
#[test]
#[ignore = "integration test"]
fn sdk_installation_test() {
    TestRunner::default().build(
        default_build_config( "tests/fixtures/basic_web_8.0"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                &formatdoc! {"
                    [Determining .NET SDK version]
                    Using .NET SDK version 8.0.300 (linux-arm64)
                    Downloading .NET SDK version 8.0.300 from https://download.visualstudio.microsoft.com/download/pr/54e5bb2e-bdd6-496d-8aba-4ed14658ee91/34fd7327eadad7611bded51dcda44c35/dotnet-sdk-8.0.300-linux-arm64.tar.gz
                    Verifying checksum
                    Installing .NET SDK
                "}
            );
        },
    );
}
