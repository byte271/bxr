#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ControlRegisters {
    pub cr0: u64,
    pub cr2: u64,
    pub cr3: u64,
    pub cr4: u64,
    pub efer: u64,
}

impl ControlRegisters {
    pub const CR0_PG: u64 = 1 << 31;
    pub const CR4_PAE: u64 = 1 << 5;
    pub const EFER_LME: u64 = 1 << 8;
    pub const EFER_LMA: u64 = 1 << 10;
    pub const EFER_NXE: u64 = 1 << 11;

    pub const fn paging_enabled(self) -> bool {
        (self.cr0 & Self::CR0_PG) != 0
    }

    pub const fn pae_enabled(self) -> bool {
        (self.cr4 & Self::CR4_PAE) != 0
    }

    pub const fn long_mode_enabled(self) -> bool {
        (self.efer & Self::EFER_LME) != 0
    }

    pub const fn long_mode_active(self) -> bool {
        (self.efer & Self::EFER_LMA) != 0
    }

    pub const fn nx_enabled(self) -> bool {
        (self.efer & Self::EFER_NXE) != 0
    }

    pub const fn cr3_base(self) -> u64 {
        self.cr3 & 0x000f_ffff_ffff_f000
    }

    pub fn set_page_fault_address(&mut self, addr: u64) {
        self.cr2 = addr;
    }
}
