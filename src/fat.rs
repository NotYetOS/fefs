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

        let end = (sblock.root_cluster * sblock.sector_per_cluster * BLOCK_SIZE - fat_addr) / 4;

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
            let addr = base_addr + offset;
            let cache = get_block_cache(addr, &self.device);
            for loc in (0..BLOCK_SIZE).step_by(4) {
                let ret = cache.lock().read(loc, |location: &u32| { *location });
                if ret == 0 { 
                    cluster = (offset + loc) / 4 + self.current;
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

struct FAT {
    iterator: FATIterator,
    sblock: SuperBlock,
    recycled: Vec<usize>,
}

impl FAT {
    fn new(device: &Arc<dyn BlockDevice>) -> Self {
        let sblock = get_sblock(device);
        let fat = Self {
            iterator: FATIterator::new(&sblock, device),
            sblock,
            recycled: Vec::new(),
        };
        fat
    }

    fn free_clusters(&mut self, size: usize) -> Vec<usize> {
        let spc = self.sblock.sector_per_cluster;
        let num_sector = if size % BLOCK_SIZE == 0 {
            size / BLOCK_SIZE
        } else {
            size / BLOCK_SIZE + 1
        };

        let num_cluster = if num_sector % spc == 0 {
            num_sector / spc
        } else {
            num_sector / spc + 1
        };

        let mut clusters = Vec::new();
        for _ in 0..num_cluster {
            clusters.push(self.free_cluster());
        }
        clusters
    }

    fn free_cluster(&mut self) -> usize {
        match self.iterator.next() {
            Some(cluster) => cluster,
            None => panic!("no fat can be allocated"),
        }
    }

    fn allocated_clusters(&self, cluster: usize) -> Vec<usize> {
        let mut cluster = cluster;
        let mut clusters = Vec::new();
        clusters.push(cluster);

        loop {
            cluster = self.read(cluster);
            if cluster == 0x0FFFFFFF {
                break;
            } else {
                clusters.push(cluster);
            }
        }

        clusters
    }

    fn read(&self, cluster: usize) -> usize {
        let addr = self.iterator.fat_addr + cluster * 4;

        get_block_cache(addr, &self.iterator.device)
            .lock().read(0, &|cluster: &u32| {
            *cluster
        }) as usize
    }

    fn write(&mut self, cluster: usize, value: usize) {
        let addr = self.iterator.fat_addr + cluster * 4;

        get_block_cache(addr, &self.iterator.device)
            .lock().modify(0, |cluster: &mut u32| {
            *cluster = value as u32;
        });
    }

    fn alloc(&mut self, size: usize) -> Vec<usize> {
        let clusters = self.free_clusters(size);
        for idx in (0..clusters.len()).step_by(2) {
            if idx == clusters.len() - 1 {
                self.write(clusters[idx], 0x0FFFFFFF);
            } else {
                self.write(clusters[idx], clusters[idx + 1]);
            }
        }
        clusters
    }

    fn dealloc(&mut self, cluster: usize) {
        let mut clusters = self.allocated_clusters(cluster);
        for &c in clusters.iter() {
            self.write(c, 0x00000000);
        }
        self.recycled.append(&mut clusters);
    }

    fn increase(&mut self, end_cluster: usize, size: usize) -> Vec<usize> {
        let new_clusters = self.alloc(size);
        self.write(end_cluster, new_clusters[0]);
        new_clusters
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

    fn inner(&mut self) -> FAT {
        match self.inner.pop() {
            Some(fat) => fat,
            None => panic!("not init fat manager"),
        }
    }

    fn push(&mut self, fat: FAT) {
        self.inner.push(fat);
    }

    fn init(&mut self, device: &Arc<dyn BlockDevice>) {
        self.push(FAT::new(device));
    }

    fn read(&mut self, cluster: usize) -> Vec<usize> {
        let fat = self.inner();
        let clusters = fat.allocated_clusters(cluster);
        self.push(fat);
        clusters
    }

    fn alloc(&mut self, size: usize) -> Vec<usize> {
        let mut fat = self.inner();
        let clusters = fat.alloc(size);
        self.push(fat);
        clusters
    }

    fn dealloc(&mut self, cluster: usize) {
        let mut fat = self.inner();
        fat.dealloc(cluster);
        self.push(fat);
    }

    fn increase(&mut self, end_cluster: usize, size: usize) -> Vec<usize> {
        let mut fat = self.inner();
        let new_clusters = fat.increase(end_cluster, size);
        self.push(fat);
        new_clusters
    }
}

pub fn create_fat(addr: usize, device: &Arc<dyn BlockDevice>) {
    get_block_cache(addr, device).lock().modify(0, |fat: &mut u64| {
        *fat = 0xFFFFFFFFFFFFFFFF;
    });
    get_block_cache(addr, device).lock().modify(8, |fat: &mut u32| {
        *fat = 0x0FFFFFFF;
    });
}

lazy_static! {
    pub static ref FAT_MANAGER: Mutex<FATManager> = Mutex::new(FATManager::new());
}

pub fn init_fat_manager(device: &Arc<dyn BlockDevice>) {
    FAT_MANAGER.lock().init(device)
}

pub fn alloc_clusters(size: usize) -> Vec<usize> {
    FAT_MANAGER.lock().alloc(size)
}

pub fn dealloc_clusters(cluster: usize) {
    FAT_MANAGER.lock().dealloc(cluster)
}

pub fn read_clusters(cluster: usize) -> Vec<usize> {
    FAT_MANAGER.lock().read(cluster)
}

pub fn increase_cluster(cluster: usize, size: usize) -> Vec<usize> {
    FAT_MANAGER.lock().increase(cluster, size)
}
