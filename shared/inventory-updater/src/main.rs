use inventory::artifact::{Arch, Artifact, Os};
use inventory::checksum::Checksum;
use inventory::Inventory;
use keep_a_changelog_file::{ChangeGroup, Changelog};
use libherokubuildpack::inventory;
use semver::Version;
use serde::Deserialize;
use sha2::Sha512;
use std::env;
use std::fs;
use std::process;
use std::str::FromStr;

fn main() {
    let (inventory_path, changelog_path) = {
        let args: Vec<String> = env::args().collect();
        if args.len() != 3 {
            eprintln!("Usage: inventory-updater <path/to/inventory.toml> <path/to/CHANGELOG.md>");
            process::exit(1);
        }
        (args[1].clone(), args[2].clone())
    };

    let local_inventory = fs::read_to_string(&inventory_path)
        .unwrap_or_else(|e| {
            eprintln!("Error reading inventory file at '{inventory_path}': {e}");
            process::exit(1);
        })
        .parse::<Inventory<Version, Sha512, Option<()>>>()
        .unwrap_or_else(|e| {
            eprintln!("Error parsing inventory file at '{inventory_path}': {e}");
            process::exit(1);
        });

    let mut upstream_artifacts = list_upstream_artifacts();
    upstream_artifacts
        .sort_by_key(|artifact| (artifact.version.clone(), artifact.arch.to_string()));
    let remote_inventory = Inventory {
        artifacts: upstream_artifacts,
    };

    fs::write(&inventory_path, remote_inventory.to_string()).unwrap_or_else(|e| {
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

    update_changelog(
        &mut changelog,
        ChangeGroup::Added,
        difference(&remote_inventory.artifacts, &local_inventory.artifacts),
    );
    update_changelog(
        &mut changelog,
        ChangeGroup::Removed,
        difference(&local_inventory.artifacts, &remote_inventory.artifacts),
    );

    fs::write(&changelog_path, changelog.to_string()).unwrap_or_else(|e| {
        eprintln!("Failed to write to changelog: {e}");
        process::exit(1);
    });
}

/// Finds the difference between two slices.
fn difference<'a, T: Eq>(a: &'a [T], b: &'a [T]) -> Vec<&'a T> {
    a.iter().filter(|&artifact| !b.contains(artifact)).collect()
}

/// Helper function to update the changelog.
fn update_changelog(
    changelog: &mut Changelog,
    change_group: ChangeGroup,
    artifacts: Vec<&Artifact<Version, Sha512, Option<()>>>,
) {
    if !artifacts.is_empty() {
        let mut sorted_artifacts: Vec<_> = artifacts.into_iter().collect();
        sorted_artifacts.sort_by_key(|artifact| &artifact.version);
        let formatted_artifacts = sorted_artifacts
            .iter()
            .map(|artifact| format!("{} ({}-{})", artifact.version, artifact.os, artifact.arch))
            .collect::<Vec<_>>()
            .join(", ");
        changelog.unreleased.add(
            change_group,
            format!("Support for .NET SDK versions: {formatted_artifacts}."),
        );
    }
}

#[derive(Deserialize)]
struct DotNetReleaseFeed {
    releases: Vec<Release>,
}

/// Represents a single .NET release within the release feed.
#[derive(Deserialize)]
struct Release {
    sdks: Vec<Sdk>,
}

/// Represents an SDK within a .NET release.
#[derive(Deserialize)]
struct Sdk {
    version: Version,
    files: Vec<File>,
}

/// Represents a file within an SDK.
#[derive(Deserialize)]
struct File {
    rid: String,
    url: String,
    hash: String,
}

const SUPPORTED_MAJOR_VERSIONS: &[i32] = &[8, 9];

fn list_upstream_artifacts() -> Vec<Artifact<Version, Sha512, Option<()>>> {
    SUPPORTED_MAJOR_VERSIONS
        .iter()
        .flat_map(|major_version| {
            ureq::get(&format!("https://dotnetcli.blob.core.windows.net/dotnet/release-metadata/{major_version}.0/releases.json"))
                .call()
                .expect(".NET release feed should be available")
                .body_mut()
                .read_json::<DotNetReleaseFeed>()
                .expect(".NET release feed should be parsable from JSON")
                .releases
                .into_iter()
                .flat_map(|release| {
                    release.sdks.into_iter().flat_map(|sdk| {
                        sdk.files.into_iter().filter_map(move |file| {
                            let (os, arch) = match file.rid.as_str() {
                                "linux-x64" => (Os::Linux, Arch::Amd64),
                                "linux-arm64" => (Os::Linux, Arch::Arm64),
                                _ => return None,
                            };
                            Some(Artifact {
                                version: sdk.version.clone(),
                                os,
                                arch,
                                url: file.url.clone(),
                                checksum: format!("sha512:{}", file.hash)
                                    .parse::<Checksum<Sha512>>()
                                    .expect("checksum should be a valid hex-encoded SHA-512 string"),
                                metadata: None,
                            })
                        })
                    })
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_difference() {
        let local_inventory = Inventory {
            artifacts: vec![Artifact::<Version, Sha512, Option<()>> {
                version: Version::parse("1.0.0").unwrap(),
                os: Os::Linux,
                arch: Arch::Amd64,
                url: "http://example.com/sdk1".to_string(),
                checksum: format!("sha512:{}", "0".repeat(128)).parse().unwrap(),
                metadata: None,
            }],
        };

        let remote_inventory = Inventory {
            artifacts: vec![
                Artifact {
                    version: Version::parse("1.0.0").unwrap(),
                    os: Os::Linux,
                    arch: Arch::Amd64,
                    url: "http://example.com/sdk1".to_string(),
                    checksum: format!("sha512:{}", "0".repeat(128)).parse().unwrap(),
                    metadata: None,
                },
                Artifact {
                    version: Version::parse("1.1.0").unwrap(),
                    os: Os::Linux,
                    arch: Arch::Amd64,
                    url: "http://example.com/sdk2".to_string(),
                    checksum: format!("sha512:{}", "1".repeat(128)).parse().unwrap(),
                    metadata: None,
                },
            ],
        };

        let added_artifacts = difference(&remote_inventory.artifacts, &local_inventory.artifacts);
        assert_eq!(added_artifacts.len(), 1);
        assert_eq!(added_artifacts[0].version, Version::parse("1.1.0").unwrap());

        let removed_artifacts = difference(&local_inventory.artifacts, &remote_inventory.artifacts);
        assert!(removed_artifacts.is_empty());
    }
}
