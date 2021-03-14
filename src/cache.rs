use alloc::{
    collections::VecDeque, 
    sync::Arc
};
use spin::Mutex;
use lazy_static::lazy_static;

use super::BLOCK_SIZE;
use super::device::BlockDevice;

pub struct BlockCache {
    cache: [u8; BLOCK_SIZE],
    addr: usize,
    device: Arc<dyn BlockDevice>,
    modified: bool,
}

impl BlockCache {
    pub fn new(
        addr: usize,
        device: Arc<dyn BlockDevice>,
    ) -> Self {
        let mut cache = [0; BLOCK_SIZE];
        device.read(addr, &mut cache);
        Self {
            cache,
            addr,
            device,
            modified: false
        }
    }

    fn addr_of_offset(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }

    pub fn get_ref<T>(&self, offset: usize) -> &T where T: Sized {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) }
    } 

    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T where T: Sized {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE);
        self.modified = true;
        let addr = self.addr_of_offset(offset);
        unsafe { &mut *(addr as *mut T) }
    }

    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }

    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.device.write(self.addr, &self.cache);
        }
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync()
    }
}

const BLOCK_CACHE_SIZE: usize = 16;

pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new()
        }
    }

    pub fn get_block_cache(
        &mut self,
        addr: usize,
        device: &Arc<dyn BlockDevice>
    ) -> Arc<Mutex<BlockCache>> {
        match self.queue
                .iter()
                .find(|&&(_addr, _)| _addr == addr) {
            Some((_, cache)) => Arc::clone(cache),
            None => {
                if self.queue.len() == BLOCK_CACHE_SIZE {
                    match self.queue
                        .iter()
                        .enumerate()
                        .find(|(_, (_, cache))| Arc::strong_count(cache) == 1) {
                        Some((index, _)) => { self.queue.remove(index).unwrap(); },
                        None => panic!("Run out of BlockCache!")
                    }
                }

                let cache = Arc::new(Mutex::new(
                    BlockCache::new(addr, Arc::clone(device))
                ));
                self.queue.push_back((addr, Arc::clone(&cache)));
                cache
            }
        }
    }
}

lazy_static! {
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> = Mutex::new(
        BlockCacheManager::new()
    );
}

pub fn get_block_cache(
    addr: usize,
    device: &Arc<dyn BlockDevice>
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER.lock().get_block_cache(addr, device)
}