use crate::{DotnetBuildpack, DotnetBuildpackError};
use bullet_stream::global::print;
use libcnb::build::BuildContext;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, EmptyLayerCause, InvalidMetadataAction, LayerRef, LayerState,
    RestoredLayerAction,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct NugetCacheLayerMetadata {
    // Using float here due to [an issue with lifecycle's handling of integers](https://github.com/buildpacks/lifecycle/issues/884)
    restore_count: f32,
}

const MAX_NUGET_CACHE_RESTORE_COUNT: f32 = 20.0;

pub(crate) fn handle(
    context: &BuildContext<DotnetBuildpack>,
    available_at_launch: bool,
) -> Result<LayerRef<DotnetBuildpack, (), f32>, libcnb::Error<DotnetBuildpackError>> {
    let nuget_cache_layer = context.cached_layer(
        layer_name!("nuget-cache"),
        CachedLayerDefinition {
            build: true,
            launch: available_at_launch,
            invalid_metadata_action: &|_| InvalidMetadataAction::DeleteLayer,
            restored_layer_action: &|metadata: &NugetCacheLayerMetadata, _path| {
                if metadata.restore_count >= MAX_NUGET_CACHE_RESTORE_COUNT {
                    (RestoredLayerAction::DeleteLayer, metadata.restore_count)
                } else {
                    (RestoredLayerAction::KeepLayer, metadata.restore_count)
                }
            },
        },
    )?;

    nuget_cache_layer.write_metadata(NugetCacheLayerMetadata {
        restore_count: match nuget_cache_layer.state {
            LayerState::Restored { cause: count } => count + 1.0,
            LayerState::Empty { .. } => 0.0,
        },
    })?;

    if let Some(message) = match nuget_cache_layer.state {
        LayerState::Restored { .. } => Some("Reusing package cache".to_string()),
        LayerState::Empty { cause } => match cause {
            EmptyLayerCause::NewlyCreated => None,
            EmptyLayerCause::InvalidMetadataAction { .. } => {
                Some("Clearing package cache due to invalid metadata".to_string())
            }
            EmptyLayerCause::RestoredLayerAction { cause: count } => {
                Some(format!("Clearing package cache after {count} uses"))
            }
        },
    } {
        print::bullet("NuGet cache");
        print::sub_bullet(message);
    }

    Ok(nuget_cache_layer)
}
