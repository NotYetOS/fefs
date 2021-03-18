#[macro_export]
macro_rules! iter_sector {
    ($self: ident, $f: expr) => {{
        let mut exit = false;
        let mut sector_addr = 0;
        for &c in $self.clusters.iter() {
            let addr = $self.sblock.offset(c);
            for o in (0..$self.sblock.sector_per_cluster) {
                sector_addr = addr + o * BLOCK_SIZE;
                exit = get_block_cache(sector_addr, &$self.device).lock().read(0, $f);
                if exit { break; }
            }
            if exit { break; }
        }
        if exit { sector_addr } else { 0 }
    }};
}

#[macro_export]
macro_rules! iter_sector_mut {
    ($self: ident, $f: expr) => {{
        let mut exit = false;
        let mut sector_addr = 0;
        for &c in $self.clusters.iter() {
            let addr = $self.sblock.offset(c);
            for o in (0..$self.sblock.sector_per_cluster) {
                sector_addr = addr + o * BLOCK_SIZE;
                exit = get_block_cache(sector_addr, &$self.device).lock().modify(0, $f);
                if exit { break; }
            }
            if exit { break; }
        }
        if exit { sector_addr } else { 0 }
    }};
}
