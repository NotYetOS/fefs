use alloc::{sync::Arc, vec::Vec};
use alloc::string::String;

use super::fat::{
    alloc_clusters,
    read_clusters
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


macro_rules! iter_sector {
    ($self: ident, $f: expr) => {{
        let mut exit = false;
        let mut sector_addr = 0;
        for &c in $self.clusters.iter() {
            let addr = $self.sblock.offset(c);
            for o in (0..$self.sblock.sector_per_cluster * BLOCK_SIZE).step_by(BLOCK_SIZE) {
                exit = get_block_cache(addr, &$self.device).lock().read(o, $f);
                if exit { 
                    sector_addr = addr + o;
                    break; 
                }
            }
            if exit { break; }
        }
        sector_addr
    }};
}
pub enum DirError {
    NotFindDir,
    NotFindFile,
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
            _ => Err(DirError::NotFindDir)
        }
    }

    pub fn open_file(&self, file: &str) -> Result<FileEntry, DirError> {
        match self.find(file) {
            Some(inode) if inode.is_filr() => Ok(FileEntry {
                device: Arc::clone(&self.device),
                clusters: read_clusters(inode.cluster()),
                sblock: &self.sblock,
            }),
            _ => Err(DirError::NotFindFile)
        }
    }

    pub fn mkdir(&self, dir: &str) -> Result<DirEntry, DirError> {
        if is_illegal(dir) { return Err(DirError::IllegalChar) };
        let cluster = alloc_clusters(BLOCK_SIZE);

        let sector_addr = iter_sector!(self, |inode: &Inode| -> bool {
            inode.is_none()
        });

        get_block_cache(sector_addr, &self.device).lock().modify(0, &|inode: &mut Inode| {
            inode.i_type = InodeType::DirEntry;
            inode.i_name.copy_from_slice(dir.as_bytes());
            inode.i_cluster = cluster as u32;
            inode.i_pre_cluster = self.clusters[0] as u32;
        });

        Ok(DirEntry {
            device: Arc::clone(&self.device),
            clusters: read_clusters(cluster),
            sblock: &self.sblock,
        })
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

    pub fn ls(&self) -> Vec<String> {
        let mut names = Vec::new();
        iter_sector!(self, |inode: &Inode| -> bool {
            if inode.is_valid() { names.push(inode.name()) }
            inode.is_none()
        });
        names
    }
}