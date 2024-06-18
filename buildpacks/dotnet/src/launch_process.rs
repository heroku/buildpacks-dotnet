use crate::dotnet_rid::RuntimeIdentifier;
use crate::dotnet_solution::DotnetSolution;
use libcnb::data::launch::Process;

pub(crate) enum LaunchProcessError {}

pub(crate) fn solution_launch_processes(
    solution: &DotnetSolution,
    configuration: &str,
    rid: &RuntimeIdentifier,
) -> Result<Vec<Process>, LaunchProcessError> {
    todo!()
}
