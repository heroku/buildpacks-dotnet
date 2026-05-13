use inventory::Inventory;
use inventory::artifact::{Arch, Artifact, Os};
use inventory::checksum::Checksum;
use itertools::Itertools;
use keep_a_changelog_file::{ChangeGroup, Changelog};
use libherokubuildpack::inventory;
use semver::Version;
use serde::Deserialize;
use sha2::Sha512;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::process;
use std::str::FromStr;

fn main() {
    let (inventory_path, changelog_path, announcement_dir) = {
        let args: Vec<String> = env::args().collect();
        match args.len() {
            3 => (args[1].clone(), args[2].clone(), None),
            4 => (args[1].clone(), args[2].clone(), Some(args[3].clone())),
            _ => {
                eprintln!(
                    "Usage: inventory-updater <path/to/inventory.toml> <path/to/CHANGELOG.md> [<path/to/announcement-dir>]"
                );
                process::exit(1);
            }
        }
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

    let UpstreamData {
        artifacts: mut upstream_artifacts,
        sdk_runtimes,
    } = list_upstream_data();
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

    let added_artifacts = difference(&remote_inventory.artifacts, &local_inventory.artifacts);
    let removed_artifacts = difference(&local_inventory.artifacts, &remote_inventory.artifacts);

    update_changelog(&mut changelog, ChangeGroup::Added, &added_artifacts);
    update_changelog(&mut changelog, ChangeGroup::Removed, &removed_artifacts);

    fs::write(&changelog_path, changelog.to_string()).unwrap_or_else(|e| {
        eprintln!("Failed to write to changelog: {e}");
        process::exit(1);
    });

    if let Some(dir) = announcement_dir {
        let added_versions: Vec<Version> = added_artifacts
            .iter()
            .map(|artifact| artifact.version.clone())
            .sorted()
            .unique()
            .collect();

        if !added_versions.is_empty() {
            let announcement = build_announcement(&added_versions, &sdk_runtimes);
            fs::create_dir_all(&dir).unwrap_or_else(|e| {
                eprintln!("Failed to create announcement directory '{dir}': {e}");
                process::exit(1);
            });
            let title_path = format!("{dir}/title.md");
            let body_path = format!("{dir}/body.md");
            fs::write(&title_path, announcement.title).unwrap_or_else(|e| {
                eprintln!("Failed to write announcement title to '{title_path}': {e}");
                process::exit(1);
            });
            fs::write(&body_path, announcement.body).unwrap_or_else(|e| {
                eprintln!("Failed to write announcement body to '{body_path}': {e}");
                process::exit(1);
            });
        }
    }
}

/// Finds the difference between two slices.
fn difference<'a, T: Eq>(a: &'a [T], b: &'a [T]) -> Vec<&'a T> {
    a.iter().filter(|&artifact| !b.contains(artifact)).collect()
}

/// Helper function to update the changelog.
fn update_changelog(
    changelog: &mut Changelog,
    change_group: ChangeGroup,
    artifacts: &[&Artifact<Version, Sha512, Option<()>>],
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
    releases: Vec<Release>,
}

/// Represents a single .NET release within the release feed.
#[derive(Deserialize)]
struct Release {
    runtime: RuntimeComponent,
    #[serde(rename = "aspnetcore-runtime")]
    aspnetcore_runtime: RuntimeComponent,
    sdks: Vec<Sdk>,
}

/// Represents the .NET Runtime or ASP.NET Core Runtime in a release.
#[derive(Deserialize)]
struct RuntimeComponent {
    version: Version,
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

/// Runtime versions associated with an SDK release.
struct RuntimeVersions {
    runtime: Version,
    aspnetcore_runtime: Version,
}

/// Data fetched from the upstream .NET release feeds.
struct UpstreamData {
    artifacts: Vec<Artifact<Version, Sha512, Option<()>>>,
    sdk_runtimes: HashMap<Version, RuntimeVersions>,
}

const SUPPORTED_MAJOR_VERSIONS: &[i32] = &[8, 9, 10];
const REQUIRED_ARCHS: [Arch; 2] = [Arch::Amd64, Arch::Arm64];

fn list_upstream_data() -> UpstreamData {
    let mut artifacts = Vec::new();
    let mut sdk_runtimes = HashMap::new();

    for major_version in SUPPORTED_MAJOR_VERSIONS {
        let feed = ureq::get(&format!("https://dotnetcli.blob.core.windows.net/dotnet/release-metadata/{major_version}.0/releases.json"))
            .call()
            .expect(".NET release feed should be available")
            .body_mut()
            .read_json::<DotNetReleaseFeed>()
            .expect(".NET release feed should be parsable from JSON");

        for release in feed.releases {
            for sdk in release.sdks {
                sdk_runtimes.insert(
                    sdk.version.clone(),
                    RuntimeVersions {
                        runtime: release.runtime.version.clone(),
                        aspnetcore_runtime: release.aspnetcore_runtime.version.clone(),
                    },
                );

                for &arch in &REQUIRED_ARCHS {
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

                    artifacts.push(Artifact {
                        version: sdk.version.clone(),
                        os: Os::Linux,
                        arch,
                        url: file.url.clone(),
                        checksum: format!("sha512:{}", file.hash)
                            .parse::<Checksum<Sha512>>()
                            .expect("Checksum should be a valid hex-encoded SHA-512 string"),
                        metadata: None,
                    });
                }
            }
        }
    }

    UpstreamData {
        artifacts,
        sdk_runtimes,
    }
}

/// Markdown announcement describing newly added SDK versions.
struct Announcement {
    title: String,
    body: String,
}

/// Renders the announcement title and body for newly added SDK versions.
fn build_announcement(
    added_versions: &[Version],
    sdk_runtimes: &HashMap<Version, RuntimeVersions>,
) -> Announcement {
    let backtick_sdk_list = format_version_list(added_versions, "`");
    let plain_sdk_list = format_version_list(added_versions, "");

    // Group SDKs by major, then by their (runtime, aspnetcore-runtime) pair so
    // SDKs that share the same runtime versions collapse into a single sentence.
    let mut sdks_by_major_and_runtimes: BTreeMap<u64, BTreeMap<(Version, Version), Vec<Version>>> =
        BTreeMap::new();
    for version in added_versions {
        if let Some(runtimes) = sdk_runtimes.get(version) {
            sdks_by_major_and_runtimes
                .entry(version.major)
                .or_default()
                .entry((
                    runtimes.runtime.clone(),
                    runtimes.aspnetcore_runtime.clone(),
                ))
                .or_default()
                .push(version.clone());
        }
    }

    let mut sentences = Vec::new();
    for (major, runtime_pairs) in &sdks_by_major_and_runtimes {
        // When every SDK in the major shares one runtime pair, use the compact
        // ".NET X.0 SDK releases include …" form. Otherwise, name the SDKs in
        // each group so the SDK ↔ runtime mapping is explicit.
        if runtime_pairs.len() == 1 {
            let ((runtime, aspnet), _) = runtime_pairs.iter().next().expect("exactly one entry");
            sentences.push(format!(
                "The .NET {major}.0 SDKs include .NET Runtime `{runtime}` and ASP.NET Core Runtime `{aspnet}`."
            ));
        } else {
            for ((runtime, aspnet), sdks) in runtime_pairs {
                let mut sorted_sdks = sdks.clone();
                sorted_sdks.sort();
                let sdk_list = format_version_list(&sorted_sdks, "`");
                let (noun, verb) = if sorted_sdks.len() == 1 {
                    ("SDK", "includes")
                } else {
                    ("SDKs", "include")
                };
                sentences.push(format!(
                    ".NET {noun} {sdk_list} {verb} .NET Runtime `{runtime}` and ASP.NET Core Runtime `{aspnet}`."
                ));
            }
        }
    }

    let runtime_paragraph = sentences.join(" ");

    Announcement {
        title: format!(".NET SDK {plain_sdk_list} are now available\n"),
        body: format!(
            ".NET SDK {backtick_sdk_list} have been made available for builds on Heroku.\n\n{runtime_paragraph}\n\nFor additional information, please see our article on [.NET support](https://devcenter.heroku.com/articles/dotnet-heroku-support-reference).\n",
        ),
    }
}

/// Joins versions into an Oxford-comma list, wrapping each in `wrap` (e.g. backticks).
fn format_version_list(versions: &[Version], wrap: &str) -> String {
    let parts: Vec<String> = versions
        .iter()
        .map(|v| format!("{wrap}{v}{wrap}"))
        .collect();
    match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let (last, rest) = parts.split_last().expect("non-empty");
            format!("{} and {last}", rest.join(", "))
        }
    }
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

    #[test]
    fn test_build_announcement_groups_by_major_version() {
        let added = vec![
            Version::parse("8.0.127").unwrap(),
            Version::parse("8.0.421").unwrap(),
            Version::parse("9.0.117").unwrap(),
            Version::parse("9.0.314").unwrap(),
            Version::parse("10.0.108").unwrap(),
            Version::parse("10.0.204").unwrap(),
            Version::parse("10.0.300").unwrap(),
        ];

        let runtimes = HashMap::from([
            (
                Version::parse("8.0.127").unwrap(),
                RuntimeVersions {
                    runtime: Version::parse("8.0.27").unwrap(),
                    aspnetcore_runtime: Version::parse("8.0.27").unwrap(),
                },
            ),
            (
                Version::parse("8.0.421").unwrap(),
                RuntimeVersions {
                    runtime: Version::parse("8.0.27").unwrap(),
                    aspnetcore_runtime: Version::parse("8.0.27").unwrap(),
                },
            ),
            (
                Version::parse("9.0.117").unwrap(),
                RuntimeVersions {
                    runtime: Version::parse("9.0.16").unwrap(),
                    aspnetcore_runtime: Version::parse("9.0.16").unwrap(),
                },
            ),
            (
                Version::parse("9.0.314").unwrap(),
                RuntimeVersions {
                    runtime: Version::parse("9.0.16").unwrap(),
                    aspnetcore_runtime: Version::parse("9.0.16").unwrap(),
                },
            ),
            (
                Version::parse("10.0.108").unwrap(),
                RuntimeVersions {
                    runtime: Version::parse("10.0.8").unwrap(),
                    aspnetcore_runtime: Version::parse("10.0.8").unwrap(),
                },
            ),
            (
                Version::parse("10.0.204").unwrap(),
                RuntimeVersions {
                    runtime: Version::parse("10.0.8").unwrap(),
                    aspnetcore_runtime: Version::parse("10.0.8").unwrap(),
                },
            ),
            (
                Version::parse("10.0.300").unwrap(),
                RuntimeVersions {
                    runtime: Version::parse("10.0.8").unwrap(),
                    aspnetcore_runtime: Version::parse("10.0.8").unwrap(),
                },
            ),
        ]);

        let expected_title = ".NET SDK 8.0.127, 8.0.421, 9.0.117, 9.0.314, 10.0.108, 10.0.204 and 10.0.300 are now available\n";
        let expected_body = ".NET SDK `8.0.127`, `8.0.421`, `9.0.117`, `9.0.314`, `10.0.108`, `10.0.204` and `10.0.300` have been made available for builds on Heroku.\n\n\
            The .NET 8.0 SDKs include .NET Runtime `8.0.27` and ASP.NET Core Runtime `8.0.27`. \
            The .NET 9.0 SDKs include .NET Runtime `9.0.16` and ASP.NET Core Runtime `9.0.16`. \
            The .NET 10.0 SDKs include .NET Runtime `10.0.8` and ASP.NET Core Runtime `10.0.8`.\n\n\
            For additional information, please see our article on [.NET support](https://devcenter.heroku.com/articles/dotnet-heroku-support-reference).\n";

        let announcement = build_announcement(&added, &runtimes);
        assert_eq!(announcement.title, expected_title);
        assert_eq!(announcement.body, expected_body);
    }

    #[test]
    fn test_build_announcement_handles_divergent_runtimes_within_a_major() {
        let added = vec![
            Version::parse("10.0.108").unwrap(),
            Version::parse("10.0.204").unwrap(),
            Version::parse("10.0.300").unwrap(),
        ];

        let runtimes = HashMap::from([
            (
                Version::parse("10.0.108").unwrap(),
                RuntimeVersions {
                    runtime: Version::parse("10.0.7").unwrap(),
                    aspnetcore_runtime: Version::parse("10.0.7").unwrap(),
                },
            ),
            (
                Version::parse("10.0.204").unwrap(),
                RuntimeVersions {
                    runtime: Version::parse("10.0.8").unwrap(),
                    aspnetcore_runtime: Version::parse("10.0.9").unwrap(),
                },
            ),
            (
                Version::parse("10.0.300").unwrap(),
                RuntimeVersions {
                    runtime: Version::parse("10.0.8").unwrap(),
                    aspnetcore_runtime: Version::parse("10.0.9").unwrap(),
                },
            ),
        ]);

        let announcement = build_announcement(&added, &runtimes);
        assert_eq!(
            announcement.body,
            ".NET SDK `10.0.108`, `10.0.204` and `10.0.300` have been made available for builds on Heroku.\n\n\
            .NET SDK `10.0.108` includes .NET Runtime `10.0.7` and ASP.NET Core Runtime `10.0.7`. \
            .NET SDKs `10.0.204` and `10.0.300` include .NET Runtime `10.0.8` and ASP.NET Core Runtime `10.0.9`.\n\n\
            For additional information, please see our article on [.NET support](https://devcenter.heroku.com/articles/dotnet-heroku-support-reference).\n"
        );
    }
}
