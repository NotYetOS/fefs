use core::fmt::Debug;

use alloc::string::String;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum INodeType {
    NoneEntry = 0,
    DirEntry = 1,
    FileEntry = 2,
}

impl Default for INodeType {
    fn default() -> Self {
        INodeType::NoneEntry
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct INode {
    pub(crate) i_type: INodeType,
    pub(crate) i_name: [u8; 16],
    pub(crate) i_name_len: u8,
    pub(crate) i_mode: u16,
    pub(crate) i_uid: u16,
    pub(crate) i_size_lo: u32,
    pub(crate) i_atime: u32,
    pub(crate) i_ctime: u32,
    pub(crate) i_mtime: u32,
    pub(crate) i_dtime: u32,
    pub(crate) i_gid: u16,
    pub(crate) i_links_count: u16,
    pub(crate) i_blocks_lo: u16,
    pub(crate) i_flags: u32,
    pub(crate) i_cluster: u32,
    pub(crate) i_pre_cluster: u32,
    pub(crate) i_offset: u32,
}

impl INode {
    pub(crate) fn is_dir(&self) -> bool {
        self.i_type == INodeType::DirEntry
    }

    pub(crate) fn is_file(&self) -> bool {
        self.i_type == INodeType::FileEntry
    }

    pub(crate) fn is_valid(&self) -> bool {
        !self.is_none()
    }

    pub(crate) fn is_none(&self) -> bool {
        self.i_type == INodeType::NoneEntry
    }

    pub(crate) fn name(&self) -> String {
        let len = self.i_name_len as usize;
        core::str::from_utf8(&self.i_name[0..len]).unwrap().into()
    }

    pub(crate) fn cluster(&self) -> usize {
        self.i_cluster as usize
    }
}


impl Debug for INode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("INode")
            .field("name", &self.name())
            .field("type", &self.i_type)
            .finish()
    }
}
