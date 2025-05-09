use crate::tests::default_build_config;
use indoc::indoc;
use libcnb_test::{TestRunner, assert_contains, assert_empty};

#[test]
#[ignore = "integration test"]
fn verify_installed_dotnet_runtime_dependencies() {
    TestRunner::default().build(
        default_build_config("tests/fixtures/class_library"),
        |context| {
        let command_output = context.run_shell_command(
            indoc! {r#"
                set -euo pipefail
                
                # Check all required dynamically linked libraries can be found in the run image.

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
