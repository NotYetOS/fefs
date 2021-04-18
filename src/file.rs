use super::cache::get_block_cache;
use super::device::BlockDevice;
use super::inode::INode;
use super::sblock::SuperBlock;
use super::BLOCK_SIZE;
use super::{
    iter_sector, 
    iter_sector_mut
};
use super::fat::{
    alloc_clusters, 
    dealloc_clusters
};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::min;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileError {
    SeekValueOverFlow,
}

pub enum WriteType {
    OverWritten,
    Append,
}

#[repr(C)]
struct Data {
    inner: [u8; BLOCK_SIZE],
}

impl Data {
    fn empty() -> Self {
        Data {
            inner: [0; BLOCK_SIZE],
        }
    }

    fn self_copy_from_slice(&mut self, offset: usize, buf: &[u8]) {
        self.inner[offset..offset + buf.len()].copy_from_slice(buf);
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
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn seek(&mut self, at: usize) -> Result<(), FileError> {
        if at > self.size {
            return Err(FileError::SeekValueOverFlow);
        }
        self.seek_at = at;
        Ok(())
    }

    pub fn read_to_vec(&self, buf: &mut Vec<u8>) -> Result<usize, FileError> {
        buf.clear();

        let mut idx = 0;
        let mut len = 0;
        let size = self.size;
        let seek_at = self.seek_at;

        iter_sector!(self, |data: &Data| {
            let start = idx * BLOCK_SIZE;
            let end = min((idx + 1) * BLOCK_SIZE, size - seek_at);
            buf.append(&mut data.inner[seek_at..end - start + seek_at].to_vec());
            idx += 1;
            len += end - start;
            end == size - seek_at
        });

        Ok(if len < size { len } else { size })
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, FileError> {
        let mut idx = 0;
        let mut len = 0;
        let size = self.size;
        let seek_at = self.seek_at;
        let buf_len = buf.len();

        if buf_len == 0 {
            panic!("if you use vec, you need use read_to_vec()")
        };

        iter_sector!(self, |data: &Data| {
            let start = idx * BLOCK_SIZE;
            let end = min(min((idx + 1) * BLOCK_SIZE, buf_len), size - seek_at);
            buf[start..end].copy_from_slice(&data.inner[seek_at..end - start + seek_at]);
            idx += 1;
            len += end - start;
            end == buf_len || end == size - seek_at
        });

        let ret = if len < size { len } else { size };
        self.seek_at = ret;
        Ok(ret)
    }

    pub fn write(&mut self, buf: &[u8], write_type: WriteType) -> Result<(), FileError> {
        if buf.is_empty() {
            return Ok(());
        }

        let mut idx = 0;
        let len = buf.len();

        match write_type {
            WriteType::OverWritten => {
                self.clean_data();
                dealloc_clusters(self.clusters[0]);
                self.clusters.clear();
                self.clusters = alloc_clusters(len);
                iter_sector_mut!(self, |data: &mut Data| {
                    let start = idx * BLOCK_SIZE;
                    let end = min(start + BLOCK_SIZE, len);
                    *data = Data::copy_from_slice(&buf[start..end]);
                    idx += 1;
                    end == len
                });
                self.size = len;
            }
            WriteType::Append => {
                let spc = self.sblock.sector_per_cluster;
                let bps = self.sblock.byte_per_sector;
                let bpc = bps * spc;

                let (cluster_from, wrote_size) = (self.size / bpc, self.size % bpc);
                let (sector_from, offset) = (wrote_size / bps, wrote_size % bps);

                let left_byte = min(BLOCK_SIZE - offset, len);

                get_block_cache(
                    self.sblock.offset(self.clusters[cluster_from]),
                    &self.device,
                )
                .lock()
                .modify(sector_from * BLOCK_SIZE, |data: &mut Data| {
                    data.self_copy_from_slice(offset, &buf[0..left_byte])
                });

                let mut done = left_byte == len;

                if !done {
                    for sector in sector_from + 1..spc {
                        let start = idx * BLOCK_SIZE + left_byte;
                        let end = min(start + BLOCK_SIZE, len);
                        idx += 1;
    
                        get_block_cache(
                            self.sblock.offset(self.clusters[cluster_from]),
                            &self.device,
                        )
                        .lock()
                        .modify(sector * BLOCK_SIZE, |data: &mut Data| {
                            *data = Data::copy_from_slice(&buf[start..end]);
                        });
    
                        if end == len {
                            done = true;
                            break;
                        }
                    }
                }
                
                if !done {
                    idx = 0;
                    let last_write = bpc - wrote_size;
                    let mut append_clusters = alloc_clusters(len - last_write);
                    for &cluster in append_clusters.iter() {
                        for sector in 0..spc {
                            let start = idx * BLOCK_SIZE + last_write;
                            let end = min(start + BLOCK_SIZE, len);
                            idx += 1;

                            get_block_cache(self.sblock.offset(cluster), &self.device)
                                .lock()
                                .modify(sector * BLOCK_SIZE, |data: &mut Data| {
                                    *data = Data::copy_from_slice(&buf[start..end]);
                                });
                            if end == len {
                                done = true;
                                break;
                            }
                        }
                        if done { break; }
                    }
                    self.clusters.append(&mut append_clusters);
                }

                self.size += len;
            }
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
        get_block_cache(self.addr, &self.device)
            .lock()
            .modify(0, |inode: &mut INode| {
                inode.i_size_lo = self.size as u32;
                inode.i_cluster = self.clusters[0] as u32;
            })
    }
}
