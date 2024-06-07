use libcnb::data::layer_name;
use libcnb::layer::UncachedLayerDefinition;
use libcnb::layer_env::Scope;
use std::fs;
use std::path::Path;

use crate::{dotnet_layer_env, DotnetBuildpack, DotnetBuildpackError};

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
        copy_recursively(sdk_layer_path.join(path), runtime_layer.path().join(path))
            .map_err(DotnetBuildpackError::CopyRuntimeFilesToRuntimeLayer)?;
    }

    Ok(())
}

fn copy_recursively<P: AsRef<Path>>(src: P, dst: P) -> std::io::Result<()> {
    if src.as_ref().is_dir() {
        fs::create_dir_all(dst.as_ref())?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.as_ref().join(entry.file_name());

            copy_recursively(&src_path, &dst_path)?;
        }
    } else {
        fs::copy(src, dst)?;
    }
    Ok(())
}
