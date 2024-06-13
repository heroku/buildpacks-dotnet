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

A solution file (e.g. `MySolution.sln`) or .NET project file (e.g. `*.csproj`, `*.vbproj` or `*.fsproj`) must be present at the root of your application's repository.

If the root directory contains both solution and project files, the solution file will be preferred for the build and publish process.

## Configuration

### .NET Version

By default, the buildpack will install the latest available .NET SDK based on the value of the [`TargetFramework` property][target-framework] in project files. The runtime version included in the SDK will also be added (along with the app itself and any published artifacts) in the final application image.

To install a different version, add a [`global.json` file][global-json] in the root directory define which .NET SDK version to install. The buildpack supports specifying both the `version` and `rollForward` policy. For instance, to install a specific version the `global.json` may look like this:

```json
{
  "sdk": {
    "version": "8.0.106",
    "rollForward": "disable"
  }
}
```

A complete inventory of supported SDK versions and platforms [is available here][inventory-toml].

[ci-badge]: https://github.com/heroku/buildpacks-dotnet/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/heroku/buildpacks-dotnet/actions/workflows/ci.yml
[cnb]: https://buildpacks.io
[heroku-buildpacks]: https://github.com/heroku/buildpacks
[pack-install]: https://buildpacks.io/docs/for-platform-operators/how-to/integrate-ci/pack/
[target-framework]: https://learn.microsoft.com/en-us/dotnet/core/project-sdk/msbuild-props#targetframework
[global-json]: https://learn.microsoft.com/en-us/dotnet/core/tools/global-json
[inventory-toml]: /buildpacks/dotnet/inventory.toml
