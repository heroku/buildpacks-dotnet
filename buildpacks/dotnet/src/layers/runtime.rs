use crate::dotnet::runtime_identifier::RuntimeIdentifier;
use crate::{dotnet_layer_env, utils, DotnetBuildpack, DotnetBuildpackError};
use libcnb::data::layer_name;
use libcnb::layer::UncachedLayerDefinition;
use libcnb::layer_env::{ModificationBehavior, Scope};
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
    runtime_identifier: &RuntimeIdentifier,
) -> Result<(), libcnb::Error<DotnetBuildpackError>> {
    let runtime_layer = context.uncached_layer(
        layer_name!("runtime"),
        UncachedLayerDefinition {
            build: false,
            launch: true,
        },
    )?;
    let mut env = dotnet_layer_env::generate_layer_env(&runtime_layer.path(), &Scope::Launch);
    env.insert(
        Scope::Launch,
        ModificationBehavior::Override,
        "HEROKU_DOTNET_RUNTIME_IDENTIFIER",
        runtime_identifier.to_string(),
    );
    runtime_layer.write_env(env)?;

    for path in RUNTIME_PATHS {
        utils::copy_recursively(sdk_layer_path.join(path), runtime_layer.path().join(path))
            .map_err(DotnetBuildpackError::CopyRuntimeFiles)?;
    }

    Ok(())
}
