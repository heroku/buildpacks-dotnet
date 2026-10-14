use libcnb::Target;
use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const XDG_RUNTIME_DIR_NAME: &str = "xdg-runtime";

pub(crate) fn xdg_runtime_dir(target: &Target, layer_path: &Path) -> Option<PathBuf> {
    (target.os == "linux" && target.distro_name == "ubuntu" && target.distro_version == "22.04")
        .then(|| layer_path.join(XDG_RUNTIME_DIR_NAME))
}

pub(crate) fn prepare_xdg_runtime_dir(path: &Path) -> Result<(), io::Error> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

/// Helper function to generate a base `LayerEnv` for .NET layers that include the .NET runtime (e.g. SDK and runtime layers).
pub(crate) fn generate_layer_env(
    layer_path: &Path,
    scope: &Scope,
    xdg_runtime_dir: Option<&Path>,
) -> LayerEnv {
    let layer_env = LayerEnv::new()
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
        // Enable detection of running in a container: https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-environment-variables#dotnet_running_in_container-and-dotnet_running_in_containers
        // This is used by a few ASP.NET Core workloads.
        // We don't need to set the (now deprecated) `DOTNET_RUNNING_IN_CONTAINER` environment variable as the framework will check for both: https://github.com/dotnet/aspnetcore/blob/8198eeb2b76305677cf94972746c2600d15ff58a/src/DataProtection/DataProtection/src/Internal/ContainerUtils.cs#L86
        .chainable_insert(
            scope.clone(),
            ModificationBehavior::Override,
            "DOTNET_RUNNING_IN_CONTAINER",
            "true",
        );

    if let Some(xdg_runtime_dir) = xdg_runtime_dir {
        // Heroku-22 uses a symlinked HOME, which .NET intentionally rejects for mutex files.
        // Preserve an application-provided XDG_RUNTIME_DIR while providing a private fallback.
        layer_env.chainable_insert(
            Scope::Launch,
            ModificationBehavior::Default,
            "XDG_RUNTIME_DIR",
            xdg_runtime_dir,
        )
    } else {
        layer_env
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils;
    use libcnb::Env;

    fn target(distro_version: &str) -> Target {
        Target {
            os: "linux".to_string(),
            arch: "amd64".to_string(),
            arch_variant: None,
            distro_name: "ubuntu".to_string(),
            distro_version: distro_version.to_string(),
        }
    }

    #[test]
    fn test_generate_dotnet_layer_env() {
        let layer_env = generate_layer_env(Path::new("/layers/sdk"), &Scope::All, None);

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
            ]
        );
    }

    #[test]
    fn xdg_runtime_dir_is_only_configured_for_ubuntu_22() {
        assert_eq!(
            xdg_runtime_dir(&target("22.04"), Path::new("/layers/runtime")),
            Some(PathBuf::from("/layers/runtime/xdg-runtime"))
        );
        assert_eq!(
            xdg_runtime_dir(&target("24.04"), Path::new("/layers/runtime")),
            None
        );
    }

    #[test]
    fn xdg_runtime_dir_defaults_without_overriding_an_existing_value() {
        let layer_env = generate_layer_env(
            Path::new("/layers/runtime"),
            &Scope::Launch,
            Some(Path::new("/layers/runtime/xdg-runtime")),
        );

        assert_eq!(
            layer_env
                .apply_to_empty(Scope::Launch)
                .get_string_lossy("XDG_RUNTIME_DIR"),
            Some("/layers/runtime/xdg-runtime".to_string())
        );

        let mut environment = Env::new();
        environment.insert("XDG_RUNTIME_DIR", "/app/runtime");
        assert_eq!(
            layer_env
                .apply(Scope::Launch, &environment)
                .get_string_lossy("XDG_RUNTIME_DIR"),
            Some("/app/runtime".to_string())
        );
    }

    #[test]
    fn prepare_xdg_runtime_dir_creates_a_private_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let runtime_dir = temp_dir.path().join(XDG_RUNTIME_DIR_NAME);

        prepare_xdg_runtime_dir(&runtime_dir).unwrap();

        assert!(runtime_dir.is_dir());
        assert_eq!(
            fs::metadata(runtime_dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }

    #[test]
    fn prepare_xdg_runtime_dir_returns_an_error_when_its_parent_is_a_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let parent = temp_dir.path().join("file");
        fs::write(&parent, "not a directory").unwrap();

        assert!(prepare_xdg_runtime_dir(&parent.join(XDG_RUNTIME_DIR_NAME)).is_err());
    }
}
