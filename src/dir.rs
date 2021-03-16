use alloc::{sync::Arc, vec::Vec};
use alloc::string::String;

use crate::fat::read_clusters;

use super::BLOCK_SIZE;

use super::inode::Inode;
use super::cache::get_block_cache;
use super::sblock::SuperBlock;
use super::device::BlockDevice;
use super::file::FileEntry;

pub struct DirEntry<'a> {
    pub(crate) device: Arc<dyn BlockDevice>,
    pub(crate) clusters: Vec<usize>,
    pub(crate) sblock: &'a SuperBlock,
}

impl<'a> DirEntry<'a> {
    pub fn cd(&self, dir: &str) -> DirEntry {
        match self.find(dir) {
            Some(inode) if inode.is_dir() => DirEntry {
                device: Arc::clone(&self.device),
                clusters: read_clusters(inode.cluster()),
                sblock: &self.sblock,
            },
            _ => panic!("not find directory")
        }
    }

    pub fn open_file(&self, file: &str) -> FileEntry {
        match self.find(file) {
            Some(inode) if inode.is_filr() => FileEntry {
                device: Arc::clone(&self.device),
                clusters: read_clusters(inode.cluster()),
                sblock: &self.sblock,
            },
            _ => panic!("not find directory")
        }
    }

    fn find(&self, name: &str) -> Option<Inode> {
        let mut ret = None;
        let mut exit = false;
        for &c in self.clusters.iter() {
            let addr = self.sblock.offset(c);
            for o in (0..self.sblock.sector_per_cluster * BLOCK_SIZE).step_by(BLOCK_SIZE) {
                get_block_cache(addr, &self.device).lock().read(o, |inode: &Inode| {
                    if inode.is_valid() && inode.name().eq(name) { 
                        ret = Some(*inode);
                        exit = true;
                    }
                    if inode.is_none() { exit = true; }
                });
                if exit { break; }
            }
            if exit { break; }
        }
        ret
    }

    pub fn ls(&self) -> Vec<String> {
        let mut names = Vec::new();
        let mut exit = false;
        for &c in self.clusters.iter() {
            let addr = self.sblock.offset(c);
            for o in (0..self.sblock.sector_per_cluster * BLOCK_SIZE).step_by(BLOCK_SIZE) {
                get_block_cache(addr, &self.device).lock().read(o, |inode: &Inode| {
                    if inode.is_valid() { names.push(inode.name()) }
                    if inode.is_none() { exit = true; }
                });
                if exit { break; }
            }
            if exit { break; }
        }
        names
    }
}
