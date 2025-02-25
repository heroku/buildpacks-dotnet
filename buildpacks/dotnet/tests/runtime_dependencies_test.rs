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

                echo "Checking required .NET dependencies..."

                OS_VERSION=$(lsb_release -rs)

                # Define common dependencies (for both Ubuntu 22.04 and 24.04)
                COMMON_DEPENDENCIES=(
                    "ca-certificates"
                    "libc6"
                    "libgcc-s1"
                    "libstdc++6"
                    "libunwind8"
                    "zlib1g"
                )

                # Define version-specific dependencies
                if [[ "$OS_VERSION" == "22.04" ]]; then
                    VERSION_SPECIFIC_DEPENDENCIES=(
                        "libgssapi-krb5-2"
                        "libicu70"
                        "liblttng-ust1"
                        "libssl3"
                    )
                elif [[ "$OS_VERSION" == "24.04" ]]; then
                    VERSION_SPECIFIC_DEPENDENCIES=(
                        "libicu74"
                        "liblttng-ust1t64"
                        "libssl3t64"
                    )
                else
                    echo "Unsupported Ubuntu version: ${OS_VERSION}. This script is designed for Ubuntu 22.04 and 24.04."
                    exit 1
                fi

                DEPENDENCIES=("${COMMON_DEPENDENCIES[@]}" "${VERSION_SPECIFIC_DEPENDENCIES[@]}")
                MISSING_PACKAGES=()

                for PACKAGE in "${DEPENDENCIES[@]}"; do
                    if ! dpkg -s "$PACKAGE" &> /dev/null; then
                        MISSING_PACKAGES+=("$PACKAGE")
                    fi
                done

                if [ ${#MISSING_PACKAGES[@]} -eq 0 ]; then
                    echo "All required .NET dependencies are installed."
                    exit 0
                else
                    echo "Missing dependencies:"
                    for PACKAGE in "${MISSING_PACKAGES[@]}"; do
                        echo "   - $PACKAGE"
                    done
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
