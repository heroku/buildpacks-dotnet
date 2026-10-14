use crate::{DotnetBuildpack, DotnetBuildpackError, dotnet_layer_env, utils};
use libcnb::data::layer_name;
use libcnb::layer::UncachedLayerDefinition;
use libcnb::layer_env::Scope;
use std::path::Path;

// These are the paths we want to copy to this layer from the SDK layer
const RUNTIME_PATHS: &[&str] = &[
    "dotnet",
    "host",
    "shared",
    "ThirdPartyNotices.txt",
    "LICENSE.txt",
];

pub(crate) fn handle(
    context: &libcnb::build::BuildContext<DotnetBuildpack>,
    sdk_layer_path: &Path,
) -> Result<(), libcnb::Error<DotnetBuildpackError>> {
    let runtime_layer = context.uncached_layer(
        layer_name!("runtime"),
        UncachedLayerDefinition {
            build: false,
            launch: true,
        },
    )?;
    let xdg_runtime_dir = dotnet_layer_env::xdg_runtime_dir(&context.target, &runtime_layer.path());
    if let Some(xdg_runtime_dir) = &xdg_runtime_dir {
        dotnet_layer_env::prepare_xdg_runtime_dir(xdg_runtime_dir)
            .map_err(DotnetBuildpackError::PrepareXdgRuntimeDir)?;
    }
    runtime_layer.write_env(dotnet_layer_env::generate_layer_env(
        &runtime_layer.path(),
        &Scope::Launch,
        xdg_runtime_dir.as_deref(),
    ))?;

    for path in RUNTIME_PATHS {
        utils::copy_recursively(sdk_layer_path.join(path), runtime_layer.path().join(path))
            .map_err(DotnetBuildpackError::CopyRuntimeFiles)?;
    }

    Ok(())
}
