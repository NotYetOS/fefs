use alloc::string::String;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum InodeType {
    NoneEntry = 0,
    DirEntry = 1,
    FileEntry = 2,
    DeletedEntry = 0xE5,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Inode {
    i_type: InodeType,
    i_name: [u8; 17],
    i_mode: u16,
    i_uid: u16,
    i_size_lo: u32,
    i_atime: u32,
    i_ctime: u32,
    i_mtime: u32,
    i_dtime: u32,
    i_gid: u16,
    i_links_count: u16,
    i_blocks_lo: u16,
    i_flags: u32,
    i_cluster: u32,
    i_pre_cluster: u32,
    i_offset: u32,
}

impl Inode {
    pub fn is_dir(&self) -> bool {
        self.i_type == InodeType::DirEntry
    }

    pub fn is_filr(&self) -> bool {
        self.i_type == InodeType::FileEntry
    }

    pub fn is_valid(&self) -> bool {
        !self.is_deleted() && !self.is_none()
    }

    pub fn is_deleted(&self) -> bool {
        self.i_type != InodeType::DeletedEntry
    }

    pub fn is_none(&self) -> bool {
        self.i_type != InodeType::NoneEntry
    }

    pub fn name(&self) -> String {
        core::str::from_utf8(&self.i_name).unwrap().into()
    }

    pub fn cluster(&self) -> usize {
        self.i_cluster as usize
    }

    pub fn pre_cluster(&self) -> usize {
        self.i_pre_cluster as usize
    }
}
