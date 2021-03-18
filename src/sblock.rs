use alloc::sync::Arc;

use super::cache::get_block_cache;
use super::device::BlockDevice;

const FEFS_MAGIC: [u8; 4] = [0x66, 0x65, 0x66, 0x73];

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SuperBlock {
    pub(crate) magic: [u8; 4],
    pub(crate) byte_per_sector: usize,
    pub(crate) sector_per_cluster: usize,
    pub(crate) sector_per_fat: usize,
    pub(crate) root_cluster: usize,
}

impl SuperBlock {
    pub fn is_valid(&self) -> bool {
        self.magic == FEFS_MAGIC
    }

    pub fn fat(&self) -> usize {
        512
    } 

    pub fn offset(&self, cluster: usize) -> usize {
        (self.sector_per_fat + (cluster - self.root_cluster) * self.sector_per_cluster)
            * self.byte_per_sector
    }
}

pub fn get_sblock(device: &Arc<dyn BlockDevice>) -> SuperBlock {
    get_block_cache(0, &Arc::clone(&device)).lock().read(0, |sblock: &SuperBlock| {
        assert!(sblock.is_valid(), "Error, Not FEFS");
        sblock.clone()
    })
}

pub fn write_sblock(sblock: SuperBlock, device: &Arc<dyn BlockDevice>) {
    get_block_cache(0, &Arc::clone(&device)).lock().modify(0, |s: &mut SuperBlock| {
        *s = sblock;
    })
}
