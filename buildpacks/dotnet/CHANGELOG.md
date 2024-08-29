# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Enhanced buildpack log output to provide more detailed information on launch process detection and registration. ([#124](https://github.com/heroku/buildpacks-dotnet/pull/124))
- Set the `PublishDir` MSBuild property to `bin/publish` when running `dotnet publish`. This change ensures that the publish output for each project is consistently placed in the same directory relative to the project file, making it easier to write `Procfile` commands that work across different OS/architectures (e.g. `linux-arm64`, `linux-x64` RIDs), build configurations (e.g. `Release`, `Debug`), and Target Framework Monikers (e.g. `net8.0`, `net6.0`). ([#121](https://github.com/heroku/buildpacks-dotnet/pull/121))

## [0.1.1] - 2024-08-19

### Added

- Support for .NET SDK versions: 8.0.401 (linux-arm64), 8.0.401 (linux-amd64).

## [0.1.0] - 2024-08-15

### Added

- Initial implementation.

[unreleased]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/heroku/buildpacks-dotnet/releases/tag/v0.1.0
