# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- A `HEROKU_DOTNET_RUNTIME_IDENTIFIER` environment variable is now set to the .NET Runtime Identifier (RID) value used to a publish an app. This enables writing `Procfile` commands that are compatible with multiple platforms and CPU architectures. ([#119](https://github.com/heroku/buildpacks-dotnet/pull/119))

## [0.1.1] - 2024-08-19

### Added

- Inventory .NET SDKs: 8.0.401 (linux-arm64), 8.0.401 (linux-amd64)

## [0.1.0] - 2024-08-15

### Added

- Initial implementation.

[unreleased]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/heroku/buildpacks-dotnet/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/heroku/buildpacks-dotnet/releases/tag/v0.1.0
