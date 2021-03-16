use alloc::{sync::Arc, vec::Vec};
use alloc::string::String;

use super::{device::BlockDevice, sblock::SuperBlock};
use super::inode::Inode;

pub struct FileEntry<'a> {
    pub(crate) device: Arc<dyn BlockDevice>,
    pub(crate) clusters: Vec<usize>,
    pub(crate) sblock: &'a SuperBlock,
}

impl<'a> FileEntry<'a> {}
