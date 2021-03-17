use alloc::{
    sync::Arc, 
    vec::Vec
};

use super::BLOCK_SIZE;
use super::device::BlockDevice;
use super::sblock::SuperBlock;
use super::inode::Inode;
use super::cache::get_block_cache;
use super::iter_sector_mut;

pub struct FileEntry<'a> {
    pub(crate) device: Arc<dyn BlockDevice>,
    pub(crate) clusters: Vec<usize>,
    pub(crate) size: usize,
    pub(crate) sblock: &'a SuperBlock,
}

impl<'a> FileEntry<'a> {
    pub fn read(&self, buf: &mut [u8]) {

    }

    pub(crate) fn clean_data(&mut self) {
        iter_sector_mut!(self, |inode: &mut Inode| {
            if inode.is_valid() {
                *inode = Inode::default();
                false
            } else {
                true
            }
        });
    }
}
