use crate::tests::default_build_config;
use indoc::indoc;
use libcnb_test::{PackResult, TestRunner, assert_contains};

#[test]
#[ignore = "integration test"]
fn detect_rejects_non_dotnet_projects() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/empty").expected_pack_result(PackResult::Failure),
        |context| {
            assert_contains!(
                context.pack_stdout,
                indoc! {"========
                    No .NET solution or project files (such as `foo.sln` or `foo.csproj`) found.
                    ======== Results ========"}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn detect_passes_with_project_toml_solution_file() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/project_toml_solution_only")
            .expected_pack_result(PackResult::Failure),
        |context| {
            // This test should pass detection but fail build (solution file doesn't exist)
            // Currently this will fail because detection doesn't support project.toml yet
            assert_contains!(context.pack_stdout, "===> DETECTING\nheroku/dotnet");
            assert_contains!(
                context.pack_stdout,
                "Using configured solution file: `MyApp.sln`"
            );
        },
    );
}
