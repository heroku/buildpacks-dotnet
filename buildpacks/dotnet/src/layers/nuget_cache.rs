use crate::DotnetBuildpack;
use libcnb::build::BuildContext;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, EmptyLayerCause, InvalidMetadataAction, LayerRef, LayerState,
    RestoredLayerAction,
};
use libcnb::Buildpack;
use libherokubuildpack::log::log_info;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct NugetCacheLayerMetadata {
    // Using float here due to [an issue with lifecycle's handling of integers](https://github.com/buildpacks/lifecycle/issues/884)
    restore_count: f32,
}

const MAX_NUGET_CACHE_RESTORE_COUNT: f32 = 10.0;

pub(crate) fn handle(
    context: &BuildContext<DotnetBuildpack>,
) -> Result<LayerRef<DotnetBuildpack, (), f32>, libcnb::Error<<DotnetBuildpack as Buildpack>::Error>>
{
    let nuget_cache_layer = context.cached_layer(
        layer_name!("nuget-cache"),
        CachedLayerDefinition {
            build: false,
            launch: false,
            invalid_metadata_action: &|_| InvalidMetadataAction::DeleteLayer,
            restored_layer_action: &|metadata: &NugetCacheLayerMetadata, _path| {
                if metadata.restore_count > MAX_NUGET_CACHE_RESTORE_COUNT {
                    (RestoredLayerAction::DeleteLayer, metadata.restore_count)
                } else {
                    (RestoredLayerAction::KeepLayer, metadata.restore_count)
                }
            },
        },
    )?;
    match nuget_cache_layer.state {
        LayerState::Restored {
            cause: restore_count,
        } => {
            log_info("Reusing NuGet package cache");
            nuget_cache_layer.write_metadata(NugetCacheLayerMetadata {
                restore_count: restore_count + 1.0,
            })?;
        }
        LayerState::Empty { cause } => {
            match cause {
                EmptyLayerCause::NewlyCreated => {
                    log_info("Created NuGet package cache");
                }
                EmptyLayerCause::InvalidMetadataAction { .. } => {
                    log_info("Purged NuGet package cache due to invalid metadata");
                }
                EmptyLayerCause::RestoredLayerAction {
                    cause: restore_count,
                } => {
                    log_info(format!(
                        "Purged NuGet package cache after {restore_count} builds"
                    ));
                }
            }
            nuget_cache_layer.write_metadata(NugetCacheLayerMetadata { restore_count: 1.0 })?;
        }
    }
    Ok(nuget_cache_layer)
}
