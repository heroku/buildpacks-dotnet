use crate::tests::{
    default_build_config, get_dotnet_arch, get_rid, replace_msbuild_log_patterns_with_placeholder,
};
use indoc::formatdoc;
use libcnb_test::{assert_contains, assert_empty, TestRunner};

#[test]
#[ignore = "integration test"]
fn test_dotnet_test_success() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/xunit_project")
          .env("DOTNET_SDK_COMMAND", "test"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                context.pack_stdout,
                "- Detected .NET file to test: `/workspace/foo.csproj`"
            );

            let rid = get_rid();
            let arch = get_dotnet_arch();
            assert_contains!(
              replace_msbuild_log_patterns_with_placeholder(&context.pack_stdout, "<PLACEHOLDER>"), 
              &formatdoc! {r#"
                - Test solution
                  - Running `dotnet test /workspace/foo.csproj --runtime {rid}`

                        Determining projects to restore...
                        Restored /workspace/foo.csproj <PLACEHOLDER>.
                        foo -> /workspace/bin/Debug/net8.0/{rid}/foo.dll
                      Test run for /workspace/bin/Debug/net8.0/{rid}/foo.dll (.NETCoreApp,Version=v8.0)
                      Microsoft (R) Test Execution Command Line Tool Version 17.8.0 ({arch})
                      Copyright (c) Microsoft Corporation.  All rights reserved.

                      Starting test execution, please wait...
                      A total of 1 test files matched the specified pattern.

                      Passed!  - Failed:     0, Passed:     1, Skipped:     0, Total:     1, Duration: < 1 ms - foo.dll (net8.0)"#}
            );
        },
    );
}

#[test]
#[ignore = "integration test"]
fn test_dotnet_test_failure() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/xunit_project_fail")
            .env("DOTNET_SDK_COMMAND", "test")
            .expected_pack_result(libcnb_test::PackResult::Failure),
        |context| {
            assert_contains!(
                context.pack_stdout,
                "- Detected .NET file to test: `/workspace/foo.csproj`"
            );
            assert_contains!(
                context.pack_stdout,
                "Failed!  - Failed:     1, Passed:     0, Skipped:     0, Total:     1, Duration: < 1 ms - foo.dll (net8.0)");
        },
    );
}
