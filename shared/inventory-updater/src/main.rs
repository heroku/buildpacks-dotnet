use inventory::Inventory;
use inventory::artifact::{Arch, Artifact, Os};
use inventory::checksum::Checksum;
use itertools::Itertools;
use keep_a_changelog_file::{ChangeGroup, Changelog};
use libherokubuildpack::inventory;
use semver::Version;
use serde::{Deserialize, Serialize};
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
        .parse::<Inventory<Version, Sha512, SdkMetadata>>()
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
        &difference(&remote_inventory.artifacts, &local_inventory.artifacts),
    );
    update_changelog(
        &mut changelog,
        ChangeGroup::Removed,
        &difference(&local_inventory.artifacts, &remote_inventory.artifacts),
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
    artifacts: &[&Artifact<Version, Sha512, SdkMetadata>],
) {
    if !artifacts.is_empty() {
        let mut versions = artifacts
            .iter()
            .map(|artifact| &artifact.version)
            .sorted()
            .unique();

        changelog.unreleased.add(
            change_group,
            format!("Support for .NET SDK versions: {}.", versions.join(", ")),
        );
    }
}

#[derive(Deserialize)]
struct DotNetReleaseFeed {
    #[serde(rename = "eol-date")]
    eol_date: Option<String>,
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

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
struct SdkMetadata {
    #[serde(with = "toml_datetime_compat", default)]
    eol_date: Option<time::OffsetDateTime>,
}

/// Parses an ISO date string (e.g., "2026-11-10") into a [`time::OffsetDateTime`] at midnight UTC.
fn parse_eol_date(s: &str) -> time::OffsetDateTime {
    let parts: Vec<&str> = s.splitn(3, '-').collect();
    assert!(
        parts.len() == 3,
        "eol-date should be in YYYY-MM-DD format: {s}"
    );
    let year: i32 = parts[0].parse().expect("year should be a valid number");
    let month: u8 = parts[1].parse().expect("month should be a valid number");
    let day: u8 = parts[2].parse().expect("day should be a valid number");
    time::Date::from_calendar_date(
        year,
        time::Month::try_from(month).expect("month should be valid"),
        day,
    )
    .expect("eol-date should be a valid calendar date")
    .midnight()
    .assume_utc()
}

const SUPPORTED_MAJOR_VERSIONS: &[i32] = &[8, 9, 10];
const REQUIRED_ARCHS: [Arch; 2] = [Arch::Amd64, Arch::Arm64];

fn list_upstream_artifacts() -> Vec<Artifact<Version, Sha512, SdkMetadata>> {
    let feeds: Vec<DotNetReleaseFeed> = SUPPORTED_MAJOR_VERSIONS
        .iter()
        .map(|major_version| {
            ureq::get(&format!("https://dotnetcli.blob.core.windows.net/dotnet/release-metadata/{major_version}.0/releases.json"))
                .call()
                .expect(".NET release feed should be available")
                .body_mut()
                .read_json::<DotNetReleaseFeed>()
                .expect(".NET release feed should be parsable from JSON")
        })
        .collect();

    feeds
        .iter()
        .flat_map(|feed| {
            let metadata = SdkMetadata {
                eol_date: feed.eol_date.as_deref().map(parse_eol_date),
            };
            feed.releases.iter().flat_map(move |release| {
                release.sdks.iter().flat_map(move |sdk| {
                    REQUIRED_ARCHS.iter().map(move |&arch| {
                        let rid = match arch {
                            Arch::Amd64 => "linux-x64",
                            Arch::Arm64 => "linux-arm64",
                        };

                        // Find the corresponding file in the SDK's file list.
                        // Panic if a required artifact is missing, as we require each version
                        // to support all required platforms.
                        let file = sdk
                            .files
                            .iter()
                            .find(|file| file.rid == rid)
                            .unwrap_or_else(|| {
                                panic!(
                                    "SDK version {} is missing the {rid} artifact for Linux.",
                                    sdk.version
                                )
                            });

                        Artifact {
                            version: sdk.version.clone(),
                            os: Os::Linux,
                            arch,
                            url: file.url.clone(),
                            checksum: format!("sha512:{}", file.hash)
                                .parse::<Checksum<Sha512>>()
                                .expect("Checksum should be a valid hex-encoded SHA-512 string"),
                            metadata,
                        }
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
            artifacts: vec![Artifact::<Version, Sha512, SdkMetadata> {
                version: Version::parse("1.0.0").unwrap(),
                os: Os::Linux,
                arch: Arch::Amd64,
                url: "http://example.com/sdk1".to_string(),
                checksum: format!("sha512:{}", "0".repeat(128)).parse().unwrap(),
                metadata: SdkMetadata { eol_date: None },
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
                    metadata: SdkMetadata { eol_date: None },
                },
                Artifact {
                    version: Version::parse("1.1.0").unwrap(),
                    os: Os::Linux,
                    arch: Arch::Amd64,
                    url: "http://example.com/sdk2".to_string(),
                    checksum: format!("sha512:{}", "1".repeat(128)).parse().unwrap(),
                    metadata: SdkMetadata { eol_date: None },
                },
            ],
        };

        let added_artifacts = difference(&remote_inventory.artifacts, &local_inventory.artifacts);
        assert_eq!(added_artifacts.len(), 1);
        assert_eq!(added_artifacts[0].version, Version::parse("1.1.0").unwrap());

        let removed_artifacts = difference(&local_inventory.artifacts, &remote_inventory.artifacts);
        assert!(removed_artifacts.is_empty());
    }

    #[test]
    fn test_parse_release_feed_with_eol_date() {
        let json = r#"{
            "eol-date": "2026-11-10",
            "releases": []
        }"#;
        let feed: DotNetReleaseFeed = serde_json::from_str(json).unwrap();
        assert_eq!(feed.eol_date.as_deref(), Some("2026-11-10"));
    }

    #[test]
    fn test_parse_release_feed_without_eol_date() {
        let json = r#"{
            "releases": []
        }"#;
        let feed: DotNetReleaseFeed = serde_json::from_str(json).unwrap();
        assert_eq!(feed.eol_date, None);
    }
}
