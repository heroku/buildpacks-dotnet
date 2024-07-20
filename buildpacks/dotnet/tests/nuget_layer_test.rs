use crate::tests::{default_build_config, replace_msbuild_log_patterns_with_placeholder};
use indoc::indoc;
use libcnb_test::{assert_contains, assert_empty, assert_not_contains, TestRunner};

#[test]
#[ignore = "integration test"]
fn test_nuget_restore_and_cache() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/console_with_nuget_package")
          .env("MSBUILD_VERBOSITY_LEVEL", "normal"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_contains!(
                replace_msbuild_log_patterns_with_placeholder(&context.pack_stdout, "<PLACEHOLDER>"),
                &indoc! {r#"
                Created NuGet package cache

                [Publish]
                MSBuild version 17.9.8+610b4d3b5 for .NET
                Build started <PLACEHOLDER>.
                     1>Project "/workspace/consoleapp.csproj" on node 1 (Restore target(s)).
                     1>_GetAllRestoreProjectPathItems:
                         Determining projects to restore...
                       Restore:
                         X.509 certificate chain validation will use the fallback certificate bundle at '/layers/heroku_dotnet/sdk/sdk/8.0.205/trustedroots/codesignctl.pem'.
                         X.509 certificate chain validation will use the fallback certificate bundle at '/layers/heroku_dotnet/sdk/sdk/8.0.205/trustedroots/timestampctl.pem'.
                         Restoring packages for /workspace/consoleapp.csproj...
                           GET https://api.nuget.org/v3-flatcontainer/newtonsoft.json/index.json
                           OK https://api.nuget.org/v3-flatcontainer/newtonsoft.json/index.json <PLACEHOLDER>
                           GET https://api.nuget.org/v3-flatcontainer/newtonsoft.json/13.0.3/newtonsoft.json.13.0.3.nupkg
                           OK https://api.nuget.org/v3-flatcontainer/newtonsoft.json/13.0.3/newtonsoft.json.13.0.3.nupkg <PLACEHOLDER>
                         Installed Newtonsoft.Json 13.0.3 from https://api.nuget.org/v3/index.json to /layers/heroku_dotnet/nuget-cache/newtonsoft.json/13.0.3 with content hash HrC5BXdl00IP9zeV+0Z848QWPAoCr9P3bDEZguI+gkLcBKAOxix/tLEAAHC+UvDNPv4a2d18lOReHMOagPa+zQ==."#}
            );

            // Verify NuGet package layer caching behavior
            let config = context.config.clone();
            context.rebuild(config, |ctx| {
                assert_not_contains!(&ctx.pack_stdout, "Installed Newtonsoft.Json 13.0.3");
                assert_contains!(&ctx.pack_stdout, "Restored /workspace/consoleapp.csproj");
            });
        });
}
