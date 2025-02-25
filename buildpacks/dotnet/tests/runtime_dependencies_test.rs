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
                ldd_output=$(find /layers -type f,l \( -name 'dotnet' -o -name '*.so*' \) -exec ldd '{}' +)
                if grep 'not found' <<<"${ldd_output}" | sort --unique; then
                    echo "The above dynamically linked libraries were not found!"
                    exit 1
                fi
            "#}
        );
        assert_empty!(command_output.stderr);
        assert_contains!(
            command_output.stdout, "All required .NET dependencies are installed");
        },
    );
}
