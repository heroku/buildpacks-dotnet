![dotnet-buildpack-github-banner](https://github.com/user-attachments/assets/8ca5a70e-0157-4cbd-90d1-d612fa5b92b7)

# Heroku Cloud Native Buildpack: .NET

[![Cloud Native Buildpacks Registry: heroku/dotnet][registry-badge]][registry-url]
[![CI on Github Actions: heroku/dotnet][ci-badge]][ci-url]

`heroku/dotnet` is the [Heroku Cloud Native Buildpack][heroku-buildpacks]
for .NET and ASP.NET Core applications. It builds .NET and ASP.NET Core application source code into application images with
minimal configuration.

> [!IMPORTANT]
> This is a [Cloud Native Buildpack][cnb], and is a component of the [Heroku Cloud Native Buildpacks][heroku-buildpacks] project, which is in preview. If you are instead looking for the Heroku Classic Buildpack for .NET (for use on the Heroku platform), you may find it [here][classic-buildpack].

## Usage

> [!NOTE]
> Before getting started, ensure you have the `pack` CLI installed. Installation instructions are available [here][pack-install].

To build a .NET application codebase into a production image:

```bash
$ cd ~/workdir/sample-dotnet-app
$ pack build sample-app --builder heroku/builder:24
```

Then run the image:
```bash
docker run --rm -it -e "PORT=8080" -p 8080:8080 sample-app
```

## Application Requirements

A solution file (e.g. `MySolution.sln`) or .NET project file (e.g. `*.csproj`, `*.vbproj` or `*.fsproj`) must be present in the application’s root directory. If the root directory contains both solution and project files, the solution file will be preferred for the build and publish process.

The buildpack support C#, Visual Basic and F# projects using the .NET and ASP.NET Core frameworks (version 8.0 and up).

## Configuration

### .NET Version

By default, the buildpack will install the latest available .NET SDK based on the value of the [`TargetFramework` property][target-framework], which must be set in each project file. TFM values that follow the `net{major_version}.0` format are currently supported (e.g. `net6.0`, `net7.0`, `net8.0`). If a solution references projects that target different framework versions, the most recent version will be preferred when inferring the .NET SDK version to install.

To install a different .NET SDK version, add a [`global.json` file][global-json] to the root directory. The buildpack supports specifying both the `version` and `rollForward` policy to define which .NET SDK version to install. For instance, to install a specific version a `global.json` file may look like this:

```json
{
  "sdk": {
    "version": "8.0.106",
    "rollForward": "disable"
  }
}
```

A complete inventory of supported .NET SDK versions and platforms [is available here](./buildpacks/dotnet/inventory.toml).

## Contributing

Issues and pull requests are welcome. See our [contributing guidelines](./CONTRIBUTING.md) if you would like to help.

[ci-badge]: https://github.com/heroku/buildpacks-dotnet/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/heroku/buildpacks-dotnet/actions/workflows/ci.yml
[classic-buildpack]: https://github.com/heroku/heroku-buildpack-dotnet
[cnb]: https://buildpacks.io
[heroku-buildpacks]: https://github.com/heroku/buildpacks
[pack-install]: https://buildpacks.io/docs/for-platform-operators/how-to/integrate-ci/pack/
[target-framework]: https://learn.microsoft.com/en-us/dotnet/core/project-sdk/msbuild-props#targetframework
[global-json]: https://learn.microsoft.com/en-us/dotnet/core/tools/global-json
[registry-badge]: https://img.shields.io/badge/dynamic/json?url=https://registry.buildpacks.io/api/v1/buildpacks/heroku/dotnet&label=version&query=$.latest.version&color=DF0A6B&logo=data:image/png;base64,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&labelColor=white
[registry-url]: https://registry.buildpacks.io/buildpacks/heroku/dotnet
