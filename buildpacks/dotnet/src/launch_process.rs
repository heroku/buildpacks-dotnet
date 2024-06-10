use libcnb::data::launch::Process;

use crate::DotnetFile;

#[derive(Debug, thiserror::Error)]
pub(crate) enum LaunchProcessError {}

impl TryFrom<&DotnetFile> for Vec<Process> {
    type Error = LaunchProcessError;

    fn try_from(value: &DotnetFile) -> Result<Self, Self::Error> {
        match value {
            DotnetFile::Solution(_) => todo!(),
            DotnetFile::Project(_) => todo!(),
        }
    }
}
