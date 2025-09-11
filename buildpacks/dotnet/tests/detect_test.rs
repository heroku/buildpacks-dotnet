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
                    No .NET application found. This buildpack requires solution (`.sln`)
                    or project (`.csproj`, `.vbproj`, `.fsproj`) files in the root directory.
                    
                    For more information, see: https://github.com/heroku/buildpacks-dotnet#application-requirements
                    ======== Results ========"}
            );
        },
    );
}
