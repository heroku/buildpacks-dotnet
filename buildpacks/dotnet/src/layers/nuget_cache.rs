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

const MAX_NUGET_CACHE_RESTORE_COUNT: f32 = 10.0;

type HandleResult = Result<
    (
        LayerRef<DotnetBuildpack, (), f32>,
        Print<state::Bullet<Stdout>>,
    ),
    libcnb::Error<DotnetBuildpackError>,
>;

pub(crate) fn handle(
    context: &BuildContext<DotnetBuildpack>,
    log: Print<state::Bullet<Stdout>>,
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

    let mut log_bullet = log.bullet("NuGet cache");

    match nuget_cache_layer.state {
        LayerState::Restored {
            cause: restore_count,
        } => {
            log_bullet = log_bullet.sub_bullet("Reusing NuGet package cache");
            nuget_cache_layer.write_metadata(NugetCacheLayerMetadata {
                restore_count: restore_count + 1.0,
            })?;
        }
        LayerState::Empty { cause } => {
            match cause {
                EmptyLayerCause::NewlyCreated => {
                    log_bullet = log_bullet.sub_bullet("Created NuGet package cache");
                }
                EmptyLayerCause::InvalidMetadataAction { .. } => {
                    log_bullet =
                        log_bullet.sub_bullet("Purged NuGet package cache due to invalid metadata");
                }
                EmptyLayerCause::RestoredLayerAction {
                    cause: restore_count,
                } => {
                    log_bullet = log_bullet.sub_bullet(format!(
                        "Purged NuGet package cache after {restore_count} builds"
                    ));
                }
            }
            nuget_cache_layer.write_metadata(NugetCacheLayerMetadata { restore_count: 1.0 })?;
        }
    }
    Ok((nuget_cache_layer, log_bullet.done()))
}
