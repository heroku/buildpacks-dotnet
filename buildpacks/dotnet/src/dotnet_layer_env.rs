use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
use std::path::Path;

/// Helper function to generate a base `LayerEnv` for .NET layers that include the .NET runtime (e.g. SDK and runtime layers).
pub(crate) fn generate_layer_env(layer_path: &Path, scope: &Scope) -> LayerEnv {
    LayerEnv::new()
        .chainable_insert(scope.clone(), ModificationBehavior::Delimiter, "PATH", ":")
        .chainable_insert(
            scope.clone(),
            ModificationBehavior::Prepend,
            "PATH",
            layer_path,
        )
        // Disable .NET tools usage collection: https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-environment-variables#dotnet_cli_telemetry_optout
        .chainable_insert(
            scope.clone(),
            ModificationBehavior::Override,
            "DOTNET_CLI_TELEMETRY_OPTOUT",
            "true",
        )
        // Using the buildpack on ARM64 Macs causes failures due to an incompatibility executing on emulated amd64 Docker images (such as builder/heroku:24).
        // This feature is disabled when executing dotnet directly on Apple Silicon (see <https://github.com/dotnet/runtime/pull/70912>).
        // The feature was opt-in for .NET 6.0, but enabled by default in later versions <https://devblogs.microsoft.com/dotnet/announcing-net-6-preview-7/#runtime-wx-write-xor-execute-support-for-all-platforms-and-architectures>.
        // This environment variable disables W^X support.
        // TODO: Investigate performance implications on platforms where this feature is supported.
        .chainable_insert(
            scope.clone(),
            ModificationBehavior::Override,
            "DOTNET_EnableWriteXorExecute",
            "0",
        )
        // Mute .NET welcome and telemetry messages: https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-environment-variables#dotnet_nologo
        .chainable_insert(
            scope.clone(),
            ModificationBehavior::Override,
            "DOTNET_NOLOGO",
            "true",
        )
        // Specify the location of .NET runtimes as they're not installed in the default location: https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-environment-variables#dotnet_root-dotnet_rootx86-dotnet_root_x86-dotnet_root_x64.
        .chainable_insert(
            scope.clone(),
            ModificationBehavior::Override,
            "DOTNET_ROOT",
            layer_path,
        )
        // .NET uses XDG_RUNTIME_DIR for named mutex files on Linux. The default home directory
        // may not be usable at launch, so use this writable buildpack layer instead.
        .chainable_insert(
            Scope::Launch,
            ModificationBehavior::Override,
            "XDG_RUNTIME_DIR",
            layer_path,
        )
        // Enable detection of running in a container: https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-environment-variables#dotnet_running_in_container-and-dotnet_running_in_containers
        // This is used by a few ASP.NET Core workloads.
        // We don't need to set the (now deprecated) `DOTNET_RUNNING_IN_CONTAINER` environment variable as the framework will check for both: https://github.com/dotnet/aspnetcore/blob/8198eeb2b76305677cf94972746c2600d15ff58a/src/DataProtection/DataProtection/src/Internal/ContainerUtils.cs#L86
        .chainable_insert(
            scope.clone(),
            ModificationBehavior::Override,
            "DOTNET_RUNNING_IN_CONTAINER",
            "true",
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils;

    #[test]
    fn test_generate_dotnet_layer_env() {
        let layer_env = generate_layer_env(Path::new("/layers/sdk"), &Scope::All);

        assert_eq!(
            utils::environment_as_sorted_vector(&layer_env.apply_to_empty(Scope::Build)),
            [
                ("DOTNET_CLI_TELEMETRY_OPTOUT", "true"),
                ("DOTNET_EnableWriteXorExecute", "0"),
                ("DOTNET_NOLOGO", "true"),
                ("DOTNET_ROOT", "/layers/sdk"),
                ("DOTNET_RUNNING_IN_CONTAINER", "true"),
                ("PATH", "/layers/sdk"),
            ]
        );
        assert_eq!(
            utils::environment_as_sorted_vector(&layer_env.apply_to_empty(Scope::Launch)),
            [
                ("DOTNET_CLI_TELEMETRY_OPTOUT", "true"),
                ("DOTNET_EnableWriteXorExecute", "0"),
                ("DOTNET_NOLOGO", "true"),
                ("DOTNET_ROOT", "/layers/sdk"),
                ("DOTNET_RUNNING_IN_CONTAINER", "true"),
                ("PATH", "/layers/sdk"),
                ("XDG_RUNTIME_DIR", "/layers/sdk"),
            ]
        );
    }
}
