use crate::tests::{default_build_config, replace_msbuild_log_patterns_with_placeholder};
use indoc::indoc;
use libcnb_test::{assert_contains, assert_empty, TestRunner};

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
                assert_contains!(replace_msbuild_log_patterns_with_placeholder(&ctx.pack_stdout, "<PLACEHOLDER>"),
                &indoc! {r#"
                Reusing cached .NET SDK version: 8.0.205
                Reusing NuGet package cache

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
                           GET https://api.nuget.org/v3/vulnerabilities/index.json
                           OK https://api.nuget.org/v3/vulnerabilities/index.json <PLACEHOLDER>
                           GET https://api.nuget.org/v3-vulnerabilities/<PLACEHOLDER>/vulnerability.base.json
                           GET https://api.nuget.org/v3-vulnerabilities/<PLACEHOLDER>/<PLACEHOLDER>/vulnerability.update.json
                           OK https://api.nuget.org/v3-vulnerabilities/<PLACEHOLDER>/vulnerability.base.json <PLACEHOLDER>
                           OK https://api.nuget.org/v3-vulnerabilities/<PLACEHOLDER>/<PLACEHOLDER>/vulnerability.update.json <PLACEHOLDER>
                         Generating MSBuild file /workspace/obj/consoleapp.csproj.nuget.g.props.
                         Generating MSBuild file /workspace/obj/consoleapp.csproj.nuget.g.targets.
                         Writing assets file to disk. Path: /workspace/obj/project.assets.json
                         Restored /workspace/consoleapp.csproj <PLACEHOLDER>.
                         
                         NuGet Config files used:
                             /home/heroku/.nuget/NuGet/NuGet.Config
                         
                         Feeds used:
                             https://api.nuget.org/v3/index.json
                     1>Done Building Project "/workspace/consoleapp.csproj" (Restore target(s))."#}
                );
            });
        });
}
