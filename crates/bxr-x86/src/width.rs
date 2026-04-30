#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Width {
    U8,
    U16,
    U32,
    U64,
}

impl Width {
    pub const fn bits(self) -> u8 {
        match self {
            Self::U8 => 8,
            Self::U16 => 16,
            Self::U32 => 32,
            Self::U64 => 64,
        }
    }

    pub const fn bytes(self) -> u8 {
        self.bits() / 8
    }

    pub const fn mask(self) -> u64 {
        match self {
            Self::U8 => 0xff,
            Self::U16 => 0xffff,
            Self::U32 => 0xffff_ffff,
            Self::U64 => u64::MAX,
        }
    }

    pub const fn sign_bit(self) -> u64 {
        1_u64 << (self.bits() - 1)
    }

    pub const fn truncate(self, value: u64) -> u64 {
        value & self.mask()
    }
}
