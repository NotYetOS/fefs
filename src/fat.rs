use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

use super::BLOCK_SIZE;
use super::cache::get_block_cache;
use super::sblock::get_sblock;
use super::sblock::SuperBlock;
use super::device::BlockDevice;

struct FATIterator {
    current: usize,
    end: usize,
    fat_addr: usize,
    device: Arc<dyn BlockDevice>,
}

impl FATIterator {
    fn new(sblock: &SuperBlock, device: &Arc<dyn BlockDevice>) -> Self {
        let mut cluster = 0;
        let fat_addr = sblock.fat();
        for offset in (0..).step_by(BLOCK_SIZE) {
            let cache = get_block_cache(fat_addr + offset, device);
            for loc in (0..BLOCK_SIZE).step_by(4) {
                let ret = cache.lock().read(loc, |location: &u32| { *location });
                if ret == 0 { 
                    cluster = offset / 4 + loc / 4;
                    break;
                };
            }
            if cluster != 0 { break; }
        }

        let end = sblock.root_cluster * sblock.sector_per_cluster * BLOCK_SIZE / 4;

        Self {
            current: cluster,
            end,
            fat_addr,
            device: Arc::clone(device),
        }
    }
}

impl Iterator for FATIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let mut cluster = 0;
        let base_addr = self.fat_addr + self.current * 4;
        for offset in (0..).step_by(BLOCK_SIZE) {
            let cache = get_block_cache(base_addr + offset, &self.device);
            for loc in (0..BLOCK_SIZE).step_by(4) {
                let ret = cache.lock().read(loc, |location: &u32| { *location });
                if ret == 0 { 
                    cluster = offset / 4 + loc / 4;
                    break;
                };
            }
            if cluster != 0 { break; }
        }

        if cluster > self.end {
            None
        } else {
            self.current = cluster;
            Some(cluster)
        }
    }
}

pub struct FAT {
    iterator: FATIterator,
    recycled: Vec<usize>,
}

impl FAT {
    fn new(device: &Arc<dyn BlockDevice>) -> Self {
        let sblock = get_sblock(device);
        let fat = Self {
            iterator: FATIterator::new(&sblock, device),
            recycled: Vec::new(),
        };
        fat
    }

    fn alloc(&mut self) -> usize {
        if let Some(cluster) = self.recycled.pop() {
            return  cluster;
        }

        let cluster = match self.iterator.next() {
            Some(cluster) => cluster,
            None => panic!("no fat can be allocated"),
        };

        let base_addr = self.iterator.fat_addr + cluster * 4;
        let addr = base_addr / BLOCK_SIZE;
        let offset = (base_addr % BLOCK_SIZE) / 4; 

        get_block_cache(addr , &self.iterator.device)
            .lock().modify(offset, |cluster: &mut u32| {
            *cluster = 0x0FFFFFFF;
        });

        cluster
    }

    fn dealloc(&mut self, cluster: usize) {
        let base_addr = self.iterator.fat_addr + cluster * 4;
        let addr = base_addr / BLOCK_SIZE;
        let offset = (base_addr % BLOCK_SIZE) / 4; 

        get_block_cache(addr , &self.iterator.device)
            .lock().modify(offset, |cluster: &mut u32| {
            *cluster = 0x00000000;
        });

        self.recycled.push(cluster);
    }
}

pub struct FATManager {
    inner: Vec<FAT>
}

impl FATManager {
    fn new() -> Self {
        Self {
            inner: Vec::new()
        }
    }

    pub fn init(&mut self, device: &Arc<dyn BlockDevice>) {
        self.inner.push(FAT::new(device));
    }

    pub fn alloc(&mut self) -> usize {
        let mut fat = self.inner.pop().unwrap();
        let cluster = fat.alloc();
        self.inner.push(fat);
        cluster
    }

    pub fn dealloc(&mut self, cluster: usize) {
        let mut fat = self.inner.pop().unwrap();
        fat.dealloc(cluster);
        self.inner.push(fat);
    }
}

lazy_static! {
    pub static ref FAT_MANAGER: Mutex<FATManager> = Mutex::new(FATManager::new());
}

