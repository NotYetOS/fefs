use alloc::sync::Arc;
use spin::Mutex;
use crate::cache::get_block_cache;

use super::sblock::SuperBlock;
use super::device::BlockDevice;
use super::inode::Inode;

pub struct FileSystem {
    device: Arc<dyn BlockDevice>,
    sblock: SuperBlock,
}

impl FileSystem {
    pub fn open(device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        let sblock = get_block_cache(0, Arc::clone(&device))
                .lock()
                .read(0, |sblock: &SuperBlock| 
        {
            assert!(sblock.is_valid(), "Error loading EFS!");
            sblock.clone()
        });

        let fs = Self {
            device,
            sblock,
        };

        Arc::new(Mutex::new(fs))
    }
}