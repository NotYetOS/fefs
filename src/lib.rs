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
pub mod macros;

pub const BLOCK_SIZE: usize = 512;

pub(crate) fn is_illegal(chs: &str) -> bool {
    let illegal_char = "\\/:*?\"<>|";
    for ch in illegal_char.chars() {
        if chs.contains(ch) {
            return true;
        }
    }
    false
}