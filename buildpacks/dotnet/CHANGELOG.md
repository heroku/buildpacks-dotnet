# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- The buildpack log output now includes more launch process detection and registration details. ([#124](https://github.com/heroku/buildpacks-dotnet/pull/124))
- The `PublishDir` MSBuild property is now set to `bin/publish` when running `dotnet publish`, to ensure that the publish output for each project is consistently sent to the same location (relative to the project file). This enables writing `Procfile` commands that are compatible with any target OS/arch (i.e. Runtime Identifier (RID)), build configuration (e.g. `Release`/`Debug`) and Target Framework Moniker (TFM). ([#120](https://github.com/heroku/buildpacks-dotnet/pull/121))

## [0.1.1] - 2024-08-19

### Added

- Inventory .NET SDKs: 8.0.401 (linux-arm64), 8.0.401 (linux-amd64)

## [0.1.0] - 2024-08-15

### Added

- Initial implementation.

[unreleased]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/heroku/buildpacks-dotnet/releases/tag/v0.1.0
