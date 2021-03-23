use core::cmp::min;
use alloc::sync::Arc;
use alloc:: vec::Vec;
use super::BLOCK_SIZE;
use super::inode::INode;
use super::device::BlockDevice;
use super::sblock::SuperBlock;
use super::cache::get_block_cache;
use super::fat::{
    alloc_clusters, 
    dealloc_clusters
};
use super::{
    iter_sector,
    iter_sector_mut
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileError {
    SeekValueOverFlow
}

pub enum WriteType {
    OverWritten,
    Append,
}

#[repr(C)]
struct Data {
    inner: [u8; BLOCK_SIZE]
}

impl Data {
    fn empty() -> Self {
        Data {
            inner: [0; BLOCK_SIZE]
        }
    }

    fn copy_from_slice(buf: &[u8]) -> Self {
        let mut data = Data::empty();
        data.inner[0..buf.len()].copy_from_slice(buf);
        data
    }
}

pub struct FileEntry {
    pub(crate) device: Arc<dyn BlockDevice>,
    pub(crate) clusters: Vec<usize>,
    pub(crate) size: usize,
    pub(crate) seek_at: usize,
    pub(crate) addr: usize,
    pub(crate) sblock: SuperBlock,
}

impl FileEntry {
    pub fn seek(&mut self, at: usize) -> Result<(), FileError> {
        if at > self.size { return Err(FileError::SeekValueOverFlow); }
        self.seek_at = at;
        Ok(())
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, FileError> {
        let mut idx = 0;
        let mut len = 0;
        let size = self.size;
        let seek_at = self.seek_at;
        let buf_len = buf.len();

        iter_sector!(self, |data: &Data| {
            let start = idx * BLOCK_SIZE;
            let end = min(min((idx + 1) * BLOCK_SIZE, buf_len), size - seek_at);
            buf[start..end].copy_from_slice(&data.inner[seek_at..end - start + seek_at]);
            idx += 1;
            len += end - start;
            end == buf_len || end == size - seek_at
        });

        Ok(if len < size { len } else { size })
    }

    pub fn write(&mut self, buf: &[u8], write_type: WriteType) -> Result<(), FileError> {
        let mut idx = 0;
        let len = buf.len();
        match write_type {
            WriteType::OverWritten => {
                self.clean_data();
                dealloc_clusters(self.clusters[0]);
                self.clusters.clear();
                self.clusters = alloc_clusters(buf.len());
                iter_sector_mut!(self, |data: &mut Data| {
                    let start = idx * BLOCK_SIZE;
                    let end = min((idx + 1) * BLOCK_SIZE, len);
                    *data = Data::copy_from_slice(&buf[start..end]);
                    idx += 1;
                    end == len
                });
                self.size = buf.len();
            }
            WriteType::Append => {}
        }

        self.update();
        Ok(())
    }

    pub(crate) fn clean_data(&mut self) {
        iter_sector_mut!(self, |data: &mut Data| {
            *data = Data::empty();
            false
        });
    }

    fn update(&mut self) {
        get_block_cache(self.addr, &self.device).lock().modify(0, |inode: &mut INode| {
            inode.i_size_lo = self.size as u32;
            inode.i_cluster = self.clusters[0] as u32;
        })
    }
}
