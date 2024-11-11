use crate::tests::default_build_config;
use libcnb_test::{assert_contains, assert_empty, assert_not_contains, TestRunner};

#[test]
#[ignore = "integration test"]
fn test_nuget_restore_and_cache() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/console_with_nuget_package")
          .env("MSBUILD_VERBOSITY_LEVEL", "normal"),
        |context| {
            assert_empty!(context.pack_stderr);
            assert_not_contains!(&context.pack_stdout, "NuGet cache");
            assert_contains!(&context.pack_stdout, "Installed Newtonsoft.Json 13.0.3 from https://api.nuget.org/v3/index.json to /layers/heroku_dotnet/nuget-cache/newtonsoft.json/13.0.3 with content hash HrC5BXdl00IP9zeV+0Z848QWPAoCr9P3bDEZguI+gkLcBKAOxix/tLEAAHC+UvDNPv4a2d18lOReHMOagPa+zQ==.");
            assert_contains!(&context.pack_stdout, "Restored /workspace/consoleapp.csproj");

            // Verify NuGet package layer caching behavior
            let config = context.config.clone();
            context.rebuild(config, |rebuild_context| {
                assert_not_contains!(&rebuild_context.pack_stdout, "Installed Newtonsoft.Json 13.0.3");
                assert_contains!(&rebuild_context.pack_stdout, "Reusing package cache");
                assert_contains!(&rebuild_context.pack_stdout, "Restored /workspace/consoleapp.csproj");
            });
        });
}
