// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use heroku_inventory_utils::checksum::Checksum;
use heroku_inventory_utils::inv::{Arch, Artifact, Inventory, Os};
use semver::Version;
use serde::Deserialize;
use sha2::Sha512;
use std::{env, fs, process};

/// Updates the local .NET SDK inventory.toml with artifacts published in the upstream feed.
fn main() {
    let inventory_path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: update_inventory <path/to/inventory.toml>");
        process::exit(2);
    });

    let remote_artifacts = list_upstream_artifacts();

    let inventory = Inventory {
        artifacts: remote_artifacts,
    };

    let toml = toml::to_string(&inventory).unwrap_or_else(|e| {
        eprintln!("Error serializing inventory as toml: {e}");
        process::exit(6);
    });

    fs::write(inventory_path, toml).unwrap_or_else(|e| {
        eprintln!("Error writing inventory to file: {e}");
        process::exit(7);
    });
}

/// Represents the .NET release feed containing multiple releases.
#[derive(Debug, Deserialize)]
struct DotNetReleaseFeed {
    releases: Vec<Release>,
}

/// Represents a single .NET release within the release feed.
#[derive(Debug, Deserialize)]
struct Release {
    sdks: Vec<Sdk>,
}

/// Represents an SDK within a .NET release.
#[derive(Debug, Deserialize)]
struct Sdk {
    version: Version,
    files: Vec<SdkFile>,
}

/// Represents a file within an SDK.
#[derive(Debug, Deserialize)]
struct SdkFile {
    hash: String,
    rid: String,
    url: String,
}

const DOTNET_UPSTREAM_RELEASE_FEED: &str =
    "https://dotnetcli.blob.core.windows.net/dotnet/release-metadata/8.0/releases.json";

fn list_upstream_artifacts() -> Vec<Artifact<Version, Sha512>> {
    ureq::get(DOTNET_UPSTREAM_RELEASE_FEED)
        .call()
        .expect(".NET release feed should be available")
        .into_json::<DotNetReleaseFeed>()
        .expect(".NET release feed to be parsable from json")
        .releases
        .iter()
        .flat_map(|release| {
            release.sdks.iter().flat_map(|sdk| {
                sdk.files.iter().filter_map(|file| {
                    let (os, arch) = match file.rid.as_str() {
                        "linux-x64" => (Os::Linux, Arch::Amd64),
                        "linux-arm64" => (Os::Linux, Arch::Arm64),
                        _ => return None,
                    };
                    Some(Artifact::<_, _> {
                        version: sdk.version.clone(),
                        os,
                        arch,
                        url: file.url.clone(),
                        checksum: Checksum::try_from(file.hash.clone())
                            .expect("checksum to be a valid hex-encoded SHA-512 string"),
                    })
                })
            })
        })
        .collect::<Vec<Artifact<_, _>>>()
}
