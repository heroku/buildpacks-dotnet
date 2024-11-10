use crate::{DotnetBuildpack, DotnetBuildpackError};
use bullet_stream::{state, Print};
use libcnb::build::BuildContext;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, EmptyLayerCause, InvalidMetadataAction, LayerRef, LayerState,
    RestoredLayerAction,
};
use serde::{Deserialize, Serialize};
use std::io::Stdout;

#[derive(Serialize, Deserialize)]
struct NugetCacheLayerMetadata {
    // Using float here due to [an issue with lifecycle's handling of integers](https://github.com/buildpacks/lifecycle/issues/884)
    restore_count: f32,
}

const MAX_NUGET_CACHE_RESTORE_COUNT: f32 = 20.0;

type HandleResult = Result<
    (
        LayerRef<DotnetBuildpack, (), f32>,
        Print<state::Bullet<Stdout>>,
    ),
    libcnb::Error<DotnetBuildpackError>,
>;

pub(crate) fn handle(
    context: &BuildContext<DotnetBuildpack>,
    mut log: Print<state::Bullet<Stdout>>,
) -> HandleResult {
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

    let log_output = match nuget_cache_layer.state {
        LayerState::Restored {
            cause: restore_count,
        } => {
            nuget_cache_layer.write_metadata(NugetCacheLayerMetadata {
                restore_count: restore_count + 1.0,
            })?;
            Some("Reusing NuGet package cache".to_string())
        }
        LayerState::Empty { cause } => {
            nuget_cache_layer.write_metadata(NugetCacheLayerMetadata { restore_count: 1.0 })?;
            match cause {
                EmptyLayerCause::NewlyCreated => None,
                EmptyLayerCause::InvalidMetadataAction { .. } => {
                    Some("Purged NuGet package cache due to invalid metadata".to_string())
                }
                EmptyLayerCause::RestoredLayerAction {
                    cause: restore_count,
                } => Some(format!(
                    "Purged NuGet package cache after {restore_count} builds"
                )),
            }
        }
    };

    if let Some(message) = log_output {
        log = log.bullet("NuGet cache").sub_bullet(message).done();
    }
    Ok((nuget_cache_layer, log))
}
