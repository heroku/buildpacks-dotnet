use crate::tests::default_build_config;
use indoc::indoc;
use libcnb_test::{assert_contains, assert_empty, PackResult, TestRunner};

#[test]
#[ignore = "integration test"]
fn test_dotnet_restore_and_run_dotnet_tool() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/console_with_dotnet_tool"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                &context.pack_stdout,
                indoc! { r"
                    - Restore .NET tools
                      - Tool manifest file detected
                      - Running `dotnet tool restore --tool-manifest /workspace/.config/dotnet-tools.json`

                          Tool 'dotnetsay' (version '2.1.7') was restored. Available commands: dotnetsay

                          Restore was successful."}
            );
            assert_contains!(&context.pack_stdout, "Running dotnetsay post-publish");
            assert_contains!(&context.pack_stdout, "__________________");
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_restore_dotnet_tool_with_configuration_error() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/console_with_dotnet_tool_configuration_error")
            .expected_pack_result(PackResult::Failure),
        |context| {
            assert_contains!(
                &context.pack_stdout,
                indoc! { r"
                    - Restore .NET tools
                      - Tool manifest file detected
                      - Running `dotnet tool restore --tool-manifest /workspace/.config/dotnet-tools.json`

                          Version 0.0.0-foobar of package dotnetsay is not found in NuGet feeds https://api.nuget.org/v3/index.json."}
            );
            assert_contains!(
                &context.pack_stderr,
                &indoc! {r"
                    ! Unable to restore .NET tools
                    !
                    ! The `dotnet tool restore` command exited unsuccessfully (exit status: 1).
                    !
                    ! This error usually happens due to configuration errors. Use the command output
                    ! above to troubleshoot and retry your build.
                    !
                    ! The .NET tool restore command can also fail for a number of other reasons, such
                    ! as intermittent network issues, unavailability of the NuGet package feed and/or
                    ! other external dependencies, etc.
                    !
                    ! Try again to see if the error resolves itself."}
            );
        },
    );
}
