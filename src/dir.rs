use alloc::sync::Arc;
use alloc::vec::Vec;
use super::BLOCK_SIZE;
use super::is_illegal;
use super::cache::get_block_cache;
use super::sblock::SuperBlock;
use super::device::BlockDevice;
use super::file::FileEntry;
use super::iter_sector;
use super::fat::{
    alloc_clusters,
    read_clusters,
    increase_cluster,
    dealloc_clusters
};
use super::inode::{
    INode,
    INodeType
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirError {
    NotFound,
    NotFoundDir,
    NotFoundFile,
    IllegalChar,
    DirExist,
    FileExist,
}

pub struct DirEntry {
    pub(crate) device: Arc<dyn BlockDevice>,
    pub(crate) clusters: Vec<usize>,
    pub(crate) sblock: SuperBlock,
}

impl DirEntry {
    pub fn cd(&self, dir: &str) -> Result<DirEntry, DirError> {
        match self.find(dir) {
            Some(inode) if inode.is_dir() => Ok(DirEntry {
                device: Arc::clone(&self.device),
                clusters: read_clusters(inode.cluster()),
                sblock: self.sblock,
            }),
            _ => Err(DirError::NotFoundDir)
        }
    }

    pub fn open_file(&self, file: &str) -> Result<FileEntry, DirError> {
        match self.find(file) {
            Some(inode) if inode.is_file() => Ok(FileEntry {
                device: Arc::clone(&self.device),
                clusters: read_clusters(inode.cluster()),
                size: inode.i_size_lo as usize,
                sblock: self.sblock,
            }),
            _ => Err(DirError::NotFoundFile)
        }
    }

    pub fn create_file(&mut self, file: &str) -> Result<FileEntry, DirError> {
        match self.find(file) {
            Some(_) => Err(DirError::FileExist),
            None if is_illegal(file) => Err(DirError::IllegalChar),
            None => Ok(FileEntry {
                device: Arc::clone(&self.device),
                clusters: self.create_inner(file, INodeType::FileEntry),
                size: 0,
                sblock: self.sblock,
            })
        }
    }

    pub fn mkdir(&mut self, dir: &str) -> Result<DirEntry, DirError> {
        match self.find(dir) {
            Some(_) => Err(DirError::DirExist),
            None if is_illegal(dir) => Err(DirError::IllegalChar),
            None => Ok(DirEntry {
                device: Arc::clone(&self.device),
                clusters: self.create_inner(dir, INodeType::DirEntry),
                sblock: self.sblock,
            })
        }
    }

    pub fn ls(&self) -> Vec<INode> {
        let mut inodes = Vec::new();
        iter_sector!(self, |inode: &INode| -> bool {
            if inode.is_valid() { inodes.push(*inode) }
            inode.is_none()
        });
        inodes
    }

    pub fn delete(&mut self, name: &str) -> Result<(), DirError> {
        match self.find_tuple(name) {
            Some((inode, addr)) => {
                match inode.i_type {
                    INodeType::NoneEntry => unreachable!(),
                    INodeType::DirEntry => DirEntry {
                        device: Arc::clone(&self.device),
                        clusters: read_clusters(inode.cluster()),
                        sblock: self.sblock,
                    }.delete_inner(),
                    INodeType::FileEntry => FileEntry {
                        device: Arc::clone(&self.device),
                        clusters: read_clusters(inode.cluster()),
                        size: inode.i_size_lo as usize,
                        sblock: self.sblock,
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
        get_block_cache(addr, &self.device).lock().modify(0, |inode: &mut INode| {
            *inode = INode::default()
        });
    }

    fn delete_inner(&mut self) {
        let inodes = self.ls();
        for (nth, inode) in inodes.iter().enumerate() {
            match inode.i_type {
                INodeType::NoneEntry => unreachable!(),
                INodeType::DirEntry => DirEntry {
                    device: Arc::clone(&self.device),
                    clusters: read_clusters(inode.cluster()),
                    sblock: self.sblock,
                }.delete_inner(),
                INodeType::FileEntry => FileEntry {
                    device: Arc::clone(&self.device),
                    clusters: read_clusters(inode.cluster()),
                    size: inode.i_size_lo as usize,
                    sblock: self.sblock,
                }.clean_data()
            }
            let index = nth / self.sblock.sector_per_cluster;
            let nth = nth % self.sblock.sector_per_cluster;
            let cluster = self.clusters[index];
            self.clean_entry(nth * BLOCK_SIZE + self.sblock.offset(cluster));
            dealloc_clusters(inode.cluster());
        }
    }

    fn find(&self, name: &str) -> Option<INode> {
        let mut ret = None;
        iter_sector!(self, |inode: &INode| -> bool {
            if inode.is_valid() && inode.name().eq(name) { 
                ret = Some(*inode);
                return true;
            }
            inode.is_none()
        });
        ret
    }

    fn find_tuple(&self, name: &str) -> Option<(INode, usize)> {
        let mut ret = INode::default();
        let addr = iter_sector!(self, |inode: &INode| -> bool {
            if inode.is_valid() && inode.name().eq(name) { 
                ret = *inode;
                return true;
            }
            inode.is_none()
        });
        if ret.is_none() {
            None
        } else {
            Some((ret, addr))
        }
    }

    fn create_inner(&mut self, name: &str, inode_type: INodeType) -> Vec<usize> {
        let clusters = alloc_clusters(BLOCK_SIZE);

        let mut sector_addr = iter_sector!(self, |inode: &INode| -> bool {
            inode.is_none()
        });

        if sector_addr == 0 {
            let clusters_len = self.clusters.len();
            let mut new_clusters = increase_cluster(self.clusters[clusters_len - 1], BLOCK_SIZE);
            self.clusters.append(&mut new_clusters);
            sector_addr = self.sblock.offset(new_clusters[0]);
        }

        get_block_cache(sector_addr, &self.device).lock().modify(0, |inode: &mut INode| {
            inode.i_type = inode_type;
            &inode.i_name[0..name.len()].copy_from_slice(name.as_bytes());
            inode.i_name_len = name.len() as u8;
            inode.i_cluster = clusters[0] as u32;
            inode.i_pre_cluster = self.clusters[0] as u32;
        });

        clusters
    }
}
