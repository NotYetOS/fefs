use alloc::{
    sync::Arc, 
    vec::Vec
};

use super::fat::{
    alloc_clusters,
    read_clusters,
    increase_cluster,
    dealloc_clusters
};
use super::inode::{
    Inode,
    InodeType
};
use super::BLOCK_SIZE;
use super::is_illegal;
use super::cache::get_block_cache;
use super::sblock::SuperBlock;
use super::device::BlockDevice;
use super::file::FileEntry;
use super::iter_sector;

pub enum DirError {
    NotFound,
    NotFoundDir,
    NotFoundFile,
    IllegalChar,
}

pub struct DirEntry<'a> {
    pub(crate) device: Arc<dyn BlockDevice>,
    pub(crate) clusters: Vec<usize>,
    pub(crate) sblock: &'a SuperBlock,
}

impl<'a> DirEntry<'a> {
    pub fn cd(&self, dir: &str) -> Result<DirEntry, DirError> {
        match self.find(dir) {
            Some(inode) if inode.is_dir() => Ok(DirEntry {
                device: Arc::clone(&self.device),
                clusters: read_clusters(inode.cluster()),
                sblock: &self.sblock,
            }),
            _ => Err(DirError::NotFoundDir)
        }
    }

    pub fn open_file(&self, file: &str) -> Result<FileEntry, DirError> {
        match self.find(file) {
            Some(inode) if inode.is_filr() => Ok(FileEntry {
                device: Arc::clone(&self.device),
                clusters: read_clusters(inode.cluster()),
                size: inode.i_size_lo as usize,
                sblock: &self.sblock,
            }),
            _ => Err(DirError::NotFoundFile)
        }
    }

    pub fn mkdir(&mut self, dir: &str) -> Result<DirEntry, DirError> {
        if is_illegal(dir) { return Err(DirError::IllegalChar) };
        let clusters = alloc_clusters(BLOCK_SIZE);

        let mut sector_addr = iter_sector!(self, |inode: &Inode| -> bool {
            inode.is_none()
        });

        if sector_addr == 0 {
            let clusters_len = self.clusters.len();
            let mut new_clusters = increase_cluster(self.clusters[clusters_len - 1], BLOCK_SIZE);
            self.clusters.append(&mut new_clusters);
            sector_addr = self.sblock.offset(new_clusters[0]);
        }

        get_block_cache(sector_addr, &self.device).lock().modify(0, |inode: &mut Inode| {
            inode.i_type = InodeType::DirEntry;
            inode.i_name.copy_from_slice(dir.as_bytes());
            inode.i_cluster = clusters[0] as u32;
            inode.i_pre_cluster = self.clusters[0] as u32;
        });

        Ok(DirEntry {
            device: Arc::clone(&self.device),
            clusters,
            sblock: &self.sblock,
        })
    }

    pub fn ls(&self) -> Vec<Inode> {
        let mut inodes = Vec::new();
        iter_sector!(self, |inode: &Inode| -> bool {
            if inode.is_valid() { inodes.push(*inode) }
            inode.is_none()
        });
        inodes
    }

    pub fn delete(&mut self, name: &str) -> Result<(), DirError> {
        match self.find_tuple(name) {
            Some((inode, addr)) => {
                match inode.i_type {
                    InodeType::NoneEntry => {}
                    InodeType::DirEntry => DirEntry {
                        device: Arc::clone(&self.device),
                        clusters: read_clusters(inode.cluster()),
                        sblock: &self.sblock,
                    }.delete_inner(),
                    InodeType::FileEntry => FileEntry {
                        device: Arc::clone(&self.device),
                        clusters: read_clusters(inode.cluster()),
                        size: inode.i_size_lo as usize,
                        sblock: &self.sblock,
                    }.clean_data()
                }
                self.clean_entry(addr);
                dealloc_clusters(inode.cluster());
                Ok(())
            },
            None => Err(DirError::NotFound)
        }
    }

    fn clean_entry(&mut self, addr: usize) {
        get_block_cache(addr, &self.device).lock().modify(0, |inode: &mut Inode| {
            *inode = Inode::default()
        });
    }

    fn delete_inner(&mut self) {
        let inodes = self.ls();
        for (nth, inode) in inodes.iter().enumerate() {
            match inode.i_type {
                InodeType::NoneEntry => {}
                InodeType::DirEntry => DirEntry {
                    device: Arc::clone(&self.device),
                    clusters: read_clusters(inode.cluster()),
                    sblock: &self.sblock,
                }.delete_inner(),
                InodeType::FileEntry => FileEntry {
                    device: Arc::clone(&self.device),
                    clusters: read_clusters(inode.cluster()),
                    size: inode.i_size_lo as usize,
                    sblock: &self.sblock,
                }.clean_data()
            }
            let index = nth / self.sblock.sector_per_cluster;
            let nth = nth % self.sblock.sector_per_cluster;
            let cluster = self.clusters[index];
            self.clean_entry(nth * BLOCK_SIZE + self.sblock.offset(cluster));
            dealloc_clusters(inode.cluster());
        }
    }

    fn find(&self, name: &str) -> Option<Inode> {
        let mut ret = None;
        iter_sector!(self, |inode: &Inode| -> bool {
            if inode.is_valid() && inode.name().eq(name) { 
                ret = Some(*inode);
                return true;
            }
            inode.is_none()
        });
        ret
    }

    fn find_tuple(&self, name: &str) -> Option<(Inode, usize)> {
        let mut ret = Inode::default();
        let addr = iter_sector!(self, |inode: &Inode| -> bool {
            if inode.is_valid() && inode.name().eq(name) { 
                ret = *inode;
                return true;
            }
            inode.is_none()
        });
        if addr == 0 {
            None
        } else {
            Some((ret, addr))
        }
    }
}
