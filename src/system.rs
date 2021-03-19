use alloc::sync::Arc;
use spin::Mutex;
use super::fat::read_clusters;
use super::dir::DirEntry;
use super::sblock::SuperBlock;
use super::device::BlockDevice;
use super::fat::{
    create_fat,
    init_fat_manager,
};
use super::sblock::{
    get_sblock,
    write_sblock,
};

pub struct FileSystem {
    device: Arc<dyn BlockDevice>,
    sblock: SuperBlock,
}

impl FileSystem {
    pub fn create(
        device: Arc<dyn BlockDevice>, 
        byte_per_sector: usize,
        sector_per_cluster: usize,
    ) -> Arc<Mutex<Self>> {
        let sblock = SuperBlock {
            magic: [0x66, 0x65, 0x66, 0x73],
            byte_per_sector,
            sector_per_cluster,
            sector_per_fat: sector_per_cluster * 2,
            root_cluster: 2,
        };
        create_fat(sblock.fat(), &device);
        write_sblock(sblock, &device);
        init_fat_manager(&device);
        Arc::new(Mutex::new(Self {
            device,
            sblock,
        }))
    }

    pub fn open(device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        let sblock = get_sblock(&device);
        init_fat_manager(&device);
        let fs = Self {
            device,
            sblock,
        };
        Arc::new(Mutex::new(fs))
    }

    pub fn root(&self) -> DirEntry {
        DirEntry {
            device: Arc::clone(&self.device),
            clusters: read_clusters(self.sblock.root_cluster),
            sblock: self.sblock,
        }
    }
}
