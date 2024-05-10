use libcnb::build::BuildResultBuilder;
use libcnb::detect::DetectResultBuilder;
use libcnb::generic::{GenericMetadata, GenericPlatform};
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
        _context: libcnb::build::BuildContext<Self>,
    ) -> libcnb::Result<libcnb::build::BuildResult, Self::Error> {
        println!("Hello, World!");
        BuildResultBuilder::new().build()
    }
}
