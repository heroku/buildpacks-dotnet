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

                          Tool 'dotnet-ef' (version '8.0.14') was restored. Available commands: dotnet-ef

                          Restore was successful."}
            );
            assert_contains!(&context.pack_stdout, "Running dotnet-ef tool post-publish");
            assert_contains!(&context.pack_stdout, "Entity Framework Core .NET Command-line Tools 8.0.14");
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_restore_dotnet_tool_test_execution_environment() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/console_with_dotnet_tool")
            .env("CNB_EXEC_ENV", "test"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(&context.pack_stdout, "Running `dotnet tool restore --tool-manifest /workspace/.config/dotnet-tools.json`");

            let command_output = context.run_shell_command("dotnet tool run dotnet-ef");
            assert_empty!(&command_output.stderr);
            assert_contains!(&command_output.stdout, "Entity Framework Core .NET Command-line Tools 8.0.14");
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
                    ! Restoring .NET tools can also fail for a number of other reasons, such as
                    ! intermittent network issues, unavailability of the NuGet package feed and/or
                    ! other external dependencies, etc.
                    !
                    ! Try again to see if the error resolves itself."}
            );
        },
    );
}
