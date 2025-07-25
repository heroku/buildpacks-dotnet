# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.8] - 2025-07-18

### Added

- Support for .NET SDK versions: 9.0.303 (linux-amd64), 9.0.303 (linux-arm64).

## [0.5.7] - 2025-07-08

### Added

- Support for .NET SDK versions: 8.0.118 (linux-amd64), 8.0.118 (linux-arm64), 8.0.315 (linux-amd64), 8.0.315 (linux-arm64), 8.0.412 (linux-amd64), 8.0.412 (linux-arm64), 9.0.108 (linux-amd64), 9.0.108 (linux-arm64), 9.0.302 (linux-amd64), 9.0.302 (linux-arm64).

## [0.5.6] - 2025-07-05

### Added

- Instrumentation of .NET SDK resolution and download operations. ([#278](https://github.com/heroku/buildpacks-dotnet/pull/278))

## [0.5.5] - 2025-06-24

### Removed

- A warning about breaking launch process type name changes (introduced in version [0.5.0](#050---2025-04-14)) is no longer written to the buildpack log output. ([#275](https://github.com/heroku/buildpacks-dotnet/pull/275))

## [0.5.4] - 2025-06-12

### Added

- Support for .NET SDK versions: 8.0.117 (linux-amd64), 8.0.117 (linux-arm64), 8.0.314 (linux-amd64), 8.0.314 (linux-arm64), 8.0.411 (linux-amd64), 8.0.411 (linux-arm64), 9.0.107 (linux-amd64), 9.0.107 (linux-arm64), 9.0.205 (linux-amd64), 9.0.205 (linux-arm64), 9.0.301 (linux-amd64), 9.0.301 (linux-arm64).

## [0.5.3] - 2025-05-26

### Added

- Support for .NET SDK versions: 8.0.313 (linux-amd64), 8.0.313 (linux-arm64), 8.0.410 (linux-amd64), 8.0.410 (linux-arm64).

## [0.5.2] - 2025-05-13

### Added

- Support for .NET SDK versions: 8.0.116 (linux-amd64), 8.0.116 (linux-arm64), 8.0.312 (linux-amd64), 8.0.312 (linux-arm64), 8.0.409 (linux-amd64), 8.0.409 (linux-arm64), 9.0.106 (linux-amd64), 9.0.106 (linux-arm64), 9.0.204 (linux-amd64), 9.0.204 (linux-arm64), 9.0.300 (linux-amd64), 9.0.300 (linux-arm64).

## [0.5.1] - 2025-05-09

### Changed

- Enhance error message for missing project references in solution files to include common causes and detailed resolution steps. ([#259](https://github.com/heroku/buildpacks-dotnet/pull/259))
- The .NET SDK inventory was updated with new release artifact download URLs. ([#263](https://github.com/heroku/buildpacks-dotnet/pull/263))

## [0.5.0] - 2025-04-14

### Changed

- The buildpack now lowercases launch process type names, and replaces spaces, dots (`.`), and underscores (`_`) with hyphens (`-`) for broader compatibility. ([#252](https://github.com/heroku/buildpacks-dotnet/pull/252))

## [0.4.1] - 2025-04-09

### Added

- Support for .NET SDK versions: 8.0.115 (linux-amd64), 8.0.115 (linux-arm64), 8.0.311 (linux-amd64), 8.0.311 (linux-arm64), 8.0.408 (linux-amd64), 8.0.408 (linux-arm64), 9.0.105 (linux-amd64), 9.0.105 (linux-arm64), 9.0.203 (linux-amd64), 9.0.203 (linux-arm64).

## [0.4.0] - 2025-03-25

### Changed

- The buildpack will now set the process type `web` for a single web application within a solution. When a solution contains multiple web applications, the process type is derived from the project assembly name. ([#242](https://github.com/heroku/buildpacks-dotnet/pull/242))

## [0.3.6] - 2025-03-20

### Changed

- The buildpack now sanitizes launch process type names, based on project assembly names, by filtering out invalid characters. ([#237](https://github.com/heroku/buildpacks-dotnet/pull/237))
- Launch process commands with paths containing special characters (including spaces) are now properly quoted. ([#239](https://github.com/heroku/buildpacks-dotnet/pull/239))
- The `test` launch process, added when targeting the test execution environment, now properly handles solution/project filenames containing special characters (including spaces). ([#240](https://github.com/heroku/buildpacks-dotnet/pull/240))

## [0.3.5] - 2025-03-19

### Added

- Support for .NET SDK versions: 9.0.202 (linux-amd64), 9.0.202 (linux-arm64).

## [0.3.4] - 2025-03-17

### Changed

- The buildpack will now restore .NET tools for any execution environment. ([#226](https://github.com/heroku/buildpacks-dotnet/pull/226))
- Restored .NET tools are now available for later buildpacks. ([#226](https://github.com/heroku/buildpacks-dotnet/pull/226))
- The log output now reflects whether a project or solution file was used for SDK version detection. ([#224](https://github.com/heroku/buildpacks-dotnet/pull/224))

## [0.3.3] - 2025-03-13

### Added

- Support for `test` and `production` execution environments. ([#222](https://github.com/heroku/buildpacks-dotnet/pull/222))

### Changed

- The NuGet cache layer is now a build layer and available for later buildpacks. ([#221](https://github.com/heroku/buildpacks-dotnet/pull/221))

## [0.3.2] - 2025-03-11

### Added

- Support for .NET SDK versions: 8.0.114 (linux-amd64), 8.0.114 (linux-arm64), 8.0.310 (linux-amd64), 8.0.310 (linux-arm64), 8.0.407 (linux-amd64), 8.0.407 (linux-arm64), 9.0.104 (linux-amd64), 9.0.104 (linux-arm64), 9.0.201 (linux-amd64), 9.0.201 (linux-arm64).

## [0.3.1] - 2025-03-10

### Changed

- The .NET SDK inventory was updated with new download URLs for version 9.0 release artifacts. ([#203](https://github.com/heroku/buildpacks-dotnet/pull/203))
- The buildpack will now skip NuGet package XML doc extraction when running `dotnet publish`. ([#212](https://github.com/heroku/buildpacks-dotnet/pull/212))
- The build configuration is no longer written to the log before the `dotnet publish` command (which still includes the build configuration value when specified). ([#213](https://github.com/heroku/buildpacks-dotnet/pull/213))

## [0.3.0] - 2025-02-28

### Changed

- The `sdk` element in detected `global.json` files is no longer required. The SDK version to install is now inferred from the solution/project files when `global.json` doesn't define SDK configuration. ([#202](https://github.com/heroku/buildpacks-dotnet/pull/202))
- The buildpack will now set `--artifacts-path` to a temporary directory during `dotnet publish`. This change reduces the number of unused, duplicated and/or intermediate files in the app directory. Published output for each project is still written to the same location relative to the the project directory (`bin/publish`, as configured using the `PublishDir` property). ([#186](https://github.com/heroku/buildpacks-dotnet/pull/186))

## [0.2.2] - 2025-02-12

### Added

- The buildpack will now restore .NET tools when a tool manifest file is detected. ([#194](https://github.com/heroku/buildpacks-dotnet/pull/194))

## [0.2.1] - 2025-02-12

### Changed

- The .NET SDK inventory was updated with new download URLs for version 9.0 release artifacts. ([#197](https://github.com/heroku/buildpacks-dotnet/pull/197))

### Added

- Support for .NET SDK versions: 8.0.113 (linux-amd64), 8.0.113 (linux-arm64), 8.0.309 (linux-amd64), 8.0.309 (linux-arm64), 8.0.406 (linux-amd64), 8.0.406 (linux-arm64), 9.0.103 (linux-amd64), 9.0.103 (linux-arm64), 9.0.200 (linux-amd64), 9.0.200 (linux-arm64). ([#197](https://github.com/heroku/buildpacks-dotnet/pull/197))

## [0.2.0] - 2025-02-10

### Changed

- Detected process types are now only registered as launch processes when no Procfile is present. ([#185](https://github.com/heroku/buildpacks-dotnet/pull/185))
- The .NET SDK inventory was updated with new download URLs for version 9.0 release artifacts. ([#193](https://github.com/heroku/buildpacks-dotnet/pull/193))

### Added

- Enabled `libcnb`'s `trace` feature. ([#184](https://github.com/heroku/buildpacks-dotnet/pull/184))

## [0.1.10] - 2025-01-15

### Changed

- Error messages are now printed to stderr. ([#173](https://github.com/heroku/buildpacks-dotnet/pull/173))

### Added

- Support for .NET SDK versions: 8.0.112 (linux-amd64), 8.0.112 (linux-arm64), 8.0.308 (linux-amd64), 8.0.308 (linux-arm64), 8.0.405 (linux-amd64), 8.0.405 (linux-arm64), 9.0.102 (linux-amd64), 9.0.102 (linux-arm64).

## [0.1.9] - 2024-12-04

### Added

- Support for .NET SDK versions: 9.0.101 (linux-amd64), 9.0.101 (linux-arm64).

## [0.1.8] - 2024-11-30

### Changed

- The buildpack will now retry SDK downloads up to 5 times ([#160](https://github.com/heroku/buildpacks-dotnet/pull/160))

## [0.1.7] - 2024-11-26

### Changed

- Web application launch processes now configure Kestrel to bind both IPv4 and IPv6 addresses. ([#156](https://github.com/heroku/buildpacks-dotnet/pull/156))

## [0.1.6] - 2024-11-12

### Added

- Support for .NET SDK versions: 8.0.111 (linux-amd64), 8.0.111 (linux-arm64), 8.0.307 (linux-amd64), 8.0.307 (linux-arm64), 8.0.404 (linux-amd64), 8.0.404 (linux-arm64), 9.0.100 (linux-amd64), 9.0.100 (linux-arm64).

## [0.1.5] - 2024-11-11

### Changed

- The buildpack will now retry SDK downloads when the request failure is caused by I/O errors. ([#140](https://github.com/heroku/buildpacks-dotnet/pull/140))

### Added

- Support for .NET SDK versions: 9.0.100-preview.1.24101.2 (linux-amd64), 9.0.100-preview.1.24101.2 (linux-arm64), 9.0.100-preview.2.24157.14 (linux-amd64), 9.0.100-preview.2.24157.14 (linux-arm64), 9.0.100-preview.3.24204.13 (linux-amd64), 9.0.100-preview.3.24204.13 (linux-arm64), 9.0.100-preview.4.24267.66 (linux-amd64), 9.0.100-preview.4.24267.66 (linux-arm64), 9.0.100-preview.5.24307.3 (linux-amd64), 9.0.100-preview.5.24307.3 (linux-arm64), 9.0.100-preview.6.24328.19 (linux-amd64), 9.0.100-preview.6.24328.19 (linux-arm64), 9.0.100-preview.7.24407.12 (linux-amd64), 9.0.100-preview.7.24407.12 (linux-arm64), 9.0.100-rc.1.24452.12 (linux-amd64), 9.0.100-rc.1.24452.12 (linux-arm64), 9.0.100-rc.2.24474.11 (linux-amd64), 9.0.100-rc.2.24474.11 (linux-arm64).

## [0.1.4] - 2024-10-09

### Added

- Support for .NET SDK versions: 8.0.110 (linux-arm64), 8.0.110 (linux-amd64), 8.0.306 (linux-arm64), 8.0.306 (linux-amd64), 8.0.403 (linux-arm64), 8.0.403 (linux-amd64).

## [0.1.3] - 2024-09-25

### Added

- Support for .NET SDK versions: 8.0.402 (linux-arm64), 8.0.402 (linux-amd64).

## [0.1.2] - 2024-08-29

### Changed

- Enhanced buildpack log output to provide more detailed information on launch process detection and registration. ([#124](https://github.com/heroku/buildpacks-dotnet/pull/124))
- Set the `PublishDir` MSBuild property to `bin/publish` when running `dotnet publish`. This change ensures that the publish output for each project is consistently placed in the same directory relative to the project file, making it easier to write `Procfile` commands that work across different OS/architectures (e.g. `linux-arm64`, `linux-x64` RIDs), build configurations (e.g. `Release`, `Debug`), and Target Framework Monikers (e.g. `net8.0`, `net6.0`). ([#121](https://github.com/heroku/buildpacks-dotnet/pull/121))

## [0.1.1] - 2024-08-19

### Added

- Support for .NET SDK versions: 8.0.401 (linux-arm64), 8.0.401 (linux-amd64).

## [0.1.0] - 2024-08-15

### Added

- Initial implementation.

[unreleased]: https://github.com/heroku/buildpacks-dotnet/compare/v0.5.8...HEAD
[0.5.8]: https://github.com/heroku/buildpacks-dotnet/compare/v0.5.7...v0.5.8
[0.5.7]: https://github.com/heroku/buildpacks-dotnet/compare/v0.5.6...v0.5.7
[0.5.6]: https://github.com/heroku/buildpacks-dotnet/compare/v0.5.5...v0.5.6
[0.5.5]: https://github.com/heroku/buildpacks-dotnet/compare/v0.5.4...v0.5.5
[0.5.4]: https://github.com/heroku/buildpacks-dotnet/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/heroku/buildpacks-dotnet/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/heroku/buildpacks-dotnet/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/heroku/buildpacks-dotnet/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/heroku/buildpacks-dotnet/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/heroku/buildpacks-dotnet/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/heroku/buildpacks-dotnet/compare/v0.3.6...v0.4.0
[0.3.6]: https://github.com/heroku/buildpacks-dotnet/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/heroku/buildpacks-dotnet/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/heroku/buildpacks-dotnet/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/heroku/buildpacks-dotnet/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/heroku/buildpacks-dotnet/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/heroku/buildpacks-dotnet/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/heroku/buildpacks-dotnet/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/heroku/buildpacks-dotnet/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/heroku/buildpacks-dotnet/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.10...v0.2.0
[0.1.10]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.9...v0.1.10
[0.1.9]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/heroku/buildpacks-dotnet/releases/tag/v0.1.0
