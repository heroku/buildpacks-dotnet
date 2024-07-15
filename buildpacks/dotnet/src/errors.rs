use crate::DotnetBuildpackError;
use indoc::formatdoc;
use libherokubuildpack::log::log_error;
use std::io;

pub(crate) fn on_error(error: libcnb::Error<DotnetBuildpackError>) {
    match error {
        libcnb::Error::BuildpackError(buildpack_error) => on_buildpack_error(&buildpack_error),
        libcnb_error => log_error(
            "Internal buildpack error",
            formatdoc! {"
                An unexpected internal error was reported by the framework used by this buildpack.
        
                If you see this error, please file an issue:
                https://github.com/heroku/buildpacks-dotnet/issues/new
        
                Details: {libcnb_error}
            "},
        ),
    }
}

fn on_buildpack_error(error: &DotnetBuildpackError) {
    log_error("A buildpack error occurred", error.to_string());
}

fn log_io_error(header: &str, occurred_while: &str, io_error: &io::Error) {
    log_error(
        header,
        formatdoc! {"
            An unexpected I/O error occurred while {occurred_while}.
            
            Details: {io_error}
        "},
    );
}
