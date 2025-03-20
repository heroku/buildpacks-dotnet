use indoc::formatdoc;
use libcnb_test::{assert_contains, assert_empty, ContainerConfig, TestRunner};

use crate::tests::{default_build_config, get_dotnet_arch};

#[test]
#[ignore = "integration test"]
fn test_restore_dotnet_tools() {
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
fn test_sdk_installation_and_launch_process() {
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

#[test]
#[ignore = "integration test"]
fn test_solution_and_project_with_spaces_in_file_and_folder_paths() {
    let mut config = default_build_config("tests/fixtures/solution_with_spaces");
    config.env("CNB_EXEC_ENV", "test");

    TestRunner::default().build(&config, |context| {
        assert_empty!(context.pack_stderr);
        context.start_container(ContainerConfig::new().entrypoint("test"), |container| {
            let log_output = container.logs_wait();

            assert_empty!(log_output.stderr);
            assert_contains!(
                log_output.stdout,
                &"Restored /workspace/console app/console app.csproj".to_string()
            );
        });
    });
}
