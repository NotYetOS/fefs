use alloc::string::String;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InodeType {
    NoneEntry = 0,
    DirEntry = 1,
    FileEntry = 2,
}

impl Default for InodeType {
    fn default() -> Self {
        InodeType::NoneEntry
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Inode {
    pub(crate) i_type: InodeType,
    pub(crate) i_name: [u8; 17],
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

impl Inode {
    pub(crate) fn is_dir(&self) -> bool {
        self.i_type == InodeType::DirEntry
    }

    pub(crate) fn is_filr(&self) -> bool {
        self.i_type == InodeType::FileEntry
    }

    pub(crate) fn is_valid(&self) -> bool {
        !self.is_none()
    }

    pub(crate) fn is_none(&self) -> bool {
        self.i_type != InodeType::NoneEntry
    }

    pub(crate) fn name(&self) -> String {
        core::str::from_utf8(&self.i_name).unwrap().into()
    }

    pub(crate) fn cluster(&self) -> usize {
        self.i_cluster as usize
    }
}
