pub(crate) mod nuget_cache;
pub(crate) mod runtime;
pub(crate) mod sdk;

pub(crate) type BuildLog = bullet_stream::Print<bullet_stream::state::Bullet<std::io::Stdout>>;
pub(crate) type LayerLogResult<T> = Result<
    (
        libcnb::layer::LayerRef<crate::DotnetBuildpack, (), T>,
        BuildLog,
    ),
    libcnb::Error<crate::DotnetBuildpackError>,
>;
