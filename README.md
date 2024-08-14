# Heroku Cloud Native Buildpack: .NET

[![CI on Github Actions: heroku/dotnet][ci-badge]][ci-url]

`heroku/dotnet` is the [Heroku Cloud Native Buildpack][heroku-buildpacks]
for .NET and ASP.NET Core applications. It builds .NET and ASP.NET Core application source code into application images with
minimal configuration.

> [!IMPORTANT]
> This is a [Cloud Native Buildpack][cnb], and is a component of the [Heroku Cloud Native Buildpacks][heroku-buildpacks] project, which is in preview.

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

By default, the buildpack will install the latest available .NET SDK based on the value of the [`TargetFramework` property][target-framework], which must be set in each project file. TFM values that follow the `net{major_version}.0` format are currently supported (e.g. `net6.0`, `net7.0`, `net8.0`).

If a solution references projects that target different framework versions, the most recent version will be preferred when inferring the .NET version. For instance, the most recent .NET 8.0 SDK release will be installed for a solution that contains a web project targeting `net8.0` and a class library targeting `net6.0`.

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
[cnb]: https://buildpacks.io
[heroku-buildpacks]: https://github.com/heroku/buildpacks
[pack-install]: https://buildpacks.io/docs/for-platform-operators/how-to/integrate-ci/pack/
[target-framework]: https://learn.microsoft.com/en-us/dotnet/core/project-sdk/msbuild-props#targetframework
[global-json]: https://learn.microsoft.com/en-us/dotnet/core/tools/global-json
