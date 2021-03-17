#[macro_export]
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

#[macro_export]
macro_rules! iter_sector_mut {
    ($self: ident, $f: expr) => {{
        let mut exit = false;
        let mut sector_addr = 0;
        for &c in $self.clusters.iter() {
            let addr = $self.sblock.offset(c);
            for o in (0..$self.sblock.sector_per_cluster * BLOCK_SIZE).step_by(BLOCK_SIZE) {
                exit = get_block_cache(addr, &$self.device).lock().modify(o, $f);
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