use libcnb::data::launch::Process;

use crate::DotnetFile;

impl TryFrom<&DotnetFile> for Vec<Process> {
    type Error = ();

    fn try_from(_value: &DotnetFile) -> Result<Self, Self::Error> {
        todo!()
    }
}
