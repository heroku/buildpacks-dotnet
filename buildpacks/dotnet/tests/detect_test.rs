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
                    No .NET application found. This buildpack requires either:
                    - .NET solution (`.sln`) or project (`.csproj`, `.vbproj`, `.fsproj`) files in the root directory
                    - A `solution_file` configured in `project.toml`
                    
                    For more information, see: https://github.com/heroku/buildpacks-dotnet#detection
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
            // Detection should pass because solution file is configured in project.toml
            // Build will fail because the configured solution file doesn't exist
            assert_contains!(context.pack_stdout, "===> DETECTING\nheroku/dotnet");
            assert_contains!(
                context.pack_stdout,
                "Using configured solution file: `MyApp.sln`"
            );
        },
    );
}
