# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[unreleased]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.4...HEAD
[0.1.4]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/heroku/buildpacks-dotnet/releases/tag/v0.1.0
