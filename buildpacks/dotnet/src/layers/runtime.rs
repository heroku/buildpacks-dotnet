use std::path::PathBuf;

use fs_extra::{copy_items, dir};
use libcnb::data::layer_name;
use libcnb::layer::UncachedLayerDefinition;
use libcnb::layer_env::Scope;

use crate::{dotnet_layer_env, DotnetBuildpack, DotnetBuildpackError};

// Copy the runtime files to it's own layer to reduce final image size.
pub(crate) fn handle(
    context: &libcnb::build::BuildContext<DotnetBuildpack>,
    sdk_layer: &libcnb::layer::LayerRef<DotnetBuildpack, (), ()>,
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

    let runtime_paths: Vec<PathBuf> = [
        "dotnet",
        "host",
        "shared",
        "ThirdPartyNotices.txt",
        "LICENSE.txt",
    ]
    .iter()
    .map(|path| sdk_layer.path().join(path))
    .collect();

    copy_items(
        &runtime_paths,
        runtime_layer.path(),
        &dir::CopyOptions {
            copy_inside: true,
            ..Default::default()
        },
    )
    .map_err(DotnetBuildpackError::CopyRuntimeFilesToRuntimeLayer)?;

    Ok(())
}
