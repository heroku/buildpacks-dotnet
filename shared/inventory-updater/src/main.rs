use inventory::artifact::{Arch, Artifact, Os};
use inventory::checksum::Checksum;
use inventory::inventory::Inventory;
use keep_a_changelog::{ChangeGroup, Changelog};
use semver::Version;
use serde::Deserialize;
use sha2::Sha512;
use std::env;
use std::fs;
use std::process;
use std::str::FromStr;

/// Updates the local .NET SDK inventory.toml with artifacts published in the upstream feed.
fn main() {
    let inventory_path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: inventory-updater <path/to/inventory.toml> <path/to/CHANGELOG.md>");
        process::exit(1);
    });

    let changelog_path = env::args().nth(2).unwrap_or_else(|| {
        eprintln!("Usage: inventory-updater <path/to/inventory.toml> <path/to/CHANGELOG.md>");
        process::exit(1);
    });

    let local_inventory: Inventory<Version, Sha512, Option<()>> = toml::from_str(
        &fs::read_to_string(inventory_path.clone()).unwrap_or_else(|e| {
            eprintln!("Error reading inventory file at '{inventory_path}': {e}");
            process::exit(1);
        }),
    )
    .unwrap_or_else(|e| {
        eprintln!("Error parsing inventory file at '{inventory_path}': {e}");
        process::exit(1);
    });

    let remote_inventory = Inventory::<Version, Sha512, Option<()>> {
        artifacts: list_upstream_artifacts(),
    };

    let toml = toml::to_string(&remote_inventory).unwrap_or_else(|e| {
        eprintln!("Error serializing inventory as toml: {e}");
        process::exit(1);
    });

    fs::write(&inventory_path, toml).unwrap_or_else(|e| {
        eprintln!("Error writing inventory to file: {e}");
        process::exit(1);
    });

    let changelog_contents = fs::read_to_string(&changelog_path).unwrap_or_else(|e| {
        eprintln!("Error reading changelog at '{changelog_path}': {e}");
        process::exit(1);
    });

    let mut changelog = Changelog::from_str(&changelog_contents).unwrap_or_else(|e| {
        eprintln!("Error parsing changelog at '{changelog_path}': {e}");
        process::exit(1);
    });

    let added_artifacts: Vec<_> = remote_inventory
        .artifacts
        .iter()
        .filter(|ra| !local_inventory.artifacts.contains(ra))
        .collect();
    let removed_artifacts: Vec<_> = local_inventory
        .artifacts
        .iter()
        .filter(|ia| !remote_inventory.artifacts.contains(ia))
        .collect();

    [
        (ChangeGroup::Added, added_artifacts),
        (ChangeGroup::Removed, removed_artifacts),
    ]
    .iter()
    .filter(|(_, artifacts)| !artifacts.is_empty())
    .for_each(|(action, artifacts)| {
        let mut list: Vec<_> = artifacts.iter().collect();
        list.sort_by_key(|a| &a.version);
        changelog.unreleased.add(
            action.clone(),
            format!(
                "Inventory .NET SDKs: {}",
                list.iter()
                    .map(|artifact| format!(
                        "{} ({}-{})",
                        artifact.version, artifact.os, artifact.arch
                    ))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        );
    });

    fs::write(&changelog_path, changelog.to_string()).unwrap_or_else(|e| {
        eprintln!("Failed to write to changelog: {e}");
        process::exit(1);
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

fn list_upstream_artifacts() -> Vec<Artifact<Version, Sha512, Option<()>>> {
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
                    Some(Artifact::<_, _, _> {
                        version: sdk.version.clone(),
                        os,
                        arch,
                        url: file.url.clone(),
                        checksum: format!("sha512:{}", file.hash)
                            .parse::<Checksum<Sha512>>()
                            .expect("checksum to be a valid hex-encoded SHA-512 string"),
                        metadata: None,
                    })
                })
            })
        })
        .collect::<Vec<Artifact<_, _, _>>>()
}
