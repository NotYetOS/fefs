const FEFS_MAGIC: [u8; 4] = [0x66, 0x65, 0x66, 0x73];

#[derive(Debug, Clone, Copy)]
pub struct SuperBlock {
    magic: [u8; 4],
    pub(crate) byte_per_sector: usize,
    pub(crate) sector_per_cluster: usize,
    pub(crate) sector_per_fat: usize,
    pub(crate) total_sector: usize,
    pub(crate) root_cluster: usize,
}

impl SuperBlock {
    pub fn is_valid(&self) -> bool {
        self.magic == FEFS_MAGIC
    }

    pub fn fat(&self) -> usize {
        core::mem::size_of::<SuperBlock>()
    } 

    pub fn offset(&self, cluster: usize) -> usize {
        (self.sector_per_fat + cluster - self.root_cluster) * self.byte_per_sector
    }
}