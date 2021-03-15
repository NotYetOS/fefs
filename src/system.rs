use alloc::sync::Arc;
use spin::Mutex;

use super::sblock::SuperBlock;
use super::device::BlockDevice;
use super::inode::Inode;
use super::sblock::get_sblock;
use super::fat::init_fat_manager;


pub struct FileSystem {
    device: Arc<dyn BlockDevice>,
    sblock: SuperBlock,
}

impl FileSystem {
    pub fn open(device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        let sblock = get_sblock(&device);
        init_fat_manager(&device);
        let fs = Self {
            device,
            sblock,
        };

        Arc::new(Mutex::new(fs))
    }
}
