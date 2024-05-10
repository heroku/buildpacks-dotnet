use libcnb::build::BuildResultBuilder;
use libcnb::data::layer_name;
use libcnb::detect::DetectResultBuilder;
use libcnb::generic::{GenericMetadata, GenericPlatform};
use libcnb::layer::{CachedLayerDefinition, InspectExistingAction, InvalidMetadataAction};
use libcnb::{buildpack_main, Buildpack};
use serde::{Deserialize, Serialize};

buildpack_main! { DotnetBuildpack }

struct DotnetBuildpack;

#[derive(thiserror::Error, Debug)]
enum DotnetBuildpackError {}

#[derive(Serialize, Deserialize)]
pub struct SdkLayerMetadata {
    sdk_version: String,
}

impl Buildpack for DotnetBuildpack {
    type Platform = GenericPlatform;

    type Metadata = GenericMetadata;

    type Error = DotnetBuildpackError;

    fn detect(
        &self,
        _context: libcnb::detect::DetectContext<Self>,
    ) -> libcnb::Result<libcnb::detect::DetectResult, Self::Error> {
        DetectResultBuilder::pass().build()
    }

    fn build(
        &self,
        context: libcnb::build::BuildContext<Self>,
    ) -> libcnb::Result<libcnb::build::BuildResult, Self::Error> {
        let _sdk_layer = context.cached_layer(
            layer_name!("sdk"),
            CachedLayerDefinition {
                build: true,
                launch: true,
                invalid_metadata: &|_| InvalidMetadataAction::DeleteLayer,
                inspect_existing: &|_metadata: &SdkLayerMetadata, _path| {
                    InspectExistingAction::Keep
                },
            },
        )?;
        println!("Hello, World!");
        BuildResultBuilder::new().build()
    }
}
