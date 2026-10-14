use crate::tests::default_build_config;
use indoc::indoc;
use libcnb_test::{TestRunner, assert_contains, assert_empty};
use std::env;

#[test]
#[ignore = "integration test"]
fn verify_installed_dotnet_runtime_dependencies() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/class_library"),
        |context| {
        if env::var("INTEGRATION_TEST_BUILDER").as_deref() == Ok("heroku/builder:22") {
            let runtime_dir_output = context.run_shell_command(
                "test \"${XDG_RUNTIME_DIR}\" = /layers/heroku_dotnet/runtime/xdg-runtime && test -d \"${XDG_RUNTIME_DIR}\" && test -w \"${XDG_RUNTIME_DIR}\" && test \"$(stat --format=%a \"${XDG_RUNTIME_DIR}\")\" = 700 && echo \"XDG runtime directory is private and writable.\"",
            );
            assert_empty!(runtime_dir_output.stderr);
            assert_contains!(
                runtime_dir_output.stdout,
                "XDG runtime directory is private and writable."
            );
        }

        let command_output = context.run_shell_command(
            indoc! {r#"
                set -euo pipefail
                
                # Check that all required dynamically linked libraries can be found in the run image.

                # Capture ldd output for `dotnet` executables and `*.so*` shared libraries in `/layers`.
                ldd_output=$(find /layers -type f,l \( -name 'dotnet' -o -name '*.so*' \) -exec ldd '{}' +)

                # Check for missing libraries.
                # An exception is made for `liblttng-ust.so.0`, which is filtered out as it is considered
                # non-critical, and expected to be missing in some environments.
                # For more info, see: https://github.com/heroku/base-images/pull/346#issuecomment-2715075259
                if grep 'not found' <<<"${ldd_output}" | grep -v 'liblttng-ust.so.0' | sort --unique; then
                    echo "The above dynamically linked libraries were not found!"
                    exit 1
                else
                    echo "All dynamically linked libraries were found."
                fi
            "#}
        );
        assert_empty!(command_output.stderr);
        assert_contains!(
            command_output.stdout, "All dynamically linked libraries were found.");
        },
    );
}
