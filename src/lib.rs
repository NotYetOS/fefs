#![no_std]
extern crate alloc;

pub mod device;
pub mod sblock;
pub mod system;
pub mod fat;
pub mod cache;
pub mod inode;
pub mod dir;
pub mod file;

pub const BLOCK_SIZE: usize = 512;
