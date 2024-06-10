use libcnb::data::launch::Process;

use crate::DotnetFile;

#[derive(Debug, thiserror::Error)]
pub(crate) enum LaunchProcessError {}

impl TryFrom<&DotnetFile> for Vec<Process> {
    type Error = LaunchProcessError;

    fn try_from(_value: &DotnetFile) -> Result<Self, Self::Error> {
        todo!()
    }
}
