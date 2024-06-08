use libcnb::data::layer_name;
use libcnb::layer::UncachedLayerDefinition;
use libcnb::layer_env::Scope;
use std::path::Path;

use crate::{dotnet_layer_env, utils, DotnetBuildpack, DotnetBuildpackError};

// These are the paths we want to this layer copy from the SDK directory/layer
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
    runtime_layer.replace_env(&dotnet_layer_env::generate_layer_env(
        &runtime_layer.path(),
        &Scope::Launch,
    ))?;

    for path in RUNTIME_PATHS {
        utils::copy_recursively(sdk_layer_path.join(path), runtime_layer.path().join(path))
            .map_err(DotnetBuildpackError::CopyRuntimeFilesToRuntimeLayer)?;
    }

    Ok(())
}