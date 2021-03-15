pub trait BlockDevice: Send + Sync + 'static {
    fn read(&self, addr: usize, buf: &mut [u8]);
    fn write(&self, addr: usize, buf: &[u8]);
}
