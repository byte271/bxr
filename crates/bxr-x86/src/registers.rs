use crate::Width;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Gpr {
    Rax = 0,
    Rcx = 1,
    Rdx = 2,
    Rbx = 3,
    Rsp = 4,
    Rbp = 5,
    Rsi = 6,
    Rdi = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
}

impl Gpr {
    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn from_low3(low3: u8, rex_extension: bool) -> Self {
        match low3 | ((rex_extension as u8) << 3) {
            0 => Self::Rax,
            1 => Self::Rcx,
            2 => Self::Rdx,
            3 => Self::Rbx,
            4 => Self::Rsp,
            5 => Self::Rbp,
            6 => Self::Rsi,
            7 => Self::Rdi,
            8 => Self::R8,
            9 => Self::R9,
            10 => Self::R10,
            11 => Self::R11,
            12 => Self::R12,
            13 => Self::R13,
            14 => Self::R14,
            _ => Self::R15,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RegisterFile {
    gprs: [u64; 16],
    rip: u64,
}

impl RegisterFile {
    pub fn read(&self, reg: Gpr) -> u64 {
        self.gprs[reg.index()]
    }

    pub fn read_width(&self, reg: Gpr, width: Width) -> u64 {
        width.truncate(self.read(reg))
    }

    pub fn write_width(&mut self, reg: Gpr, width: Width, value: u64) {
        let index = reg.index();
        let old = self.gprs[index];
        self.gprs[index] = match width {
            Width::U8 => (old & !0xff) | (value & 0xff),
            Width::U16 => (old & !0xffff) | (value & 0xffff),
            Width::U32 => value & 0xffff_ffff,
            Width::U64 => value,
        };
    }

    pub fn rip(&self) -> u64 {
        self.rip
    }

    pub fn set_rip(&mut self, rip: u64) {
        self.rip = rip;
    }

    pub fn advance_rip(&mut self, len: u8) {
        self.rip = self.rip.wrapping_add(u64::from(len));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writing_32_bits_zero_extends() {
        let mut regs = RegisterFile::default();
        regs.write_width(Gpr::Rax, Width::U64, u64::MAX);
        regs.write_width(Gpr::Rax, Width::U32, 0x1234_5678);
        assert_eq!(regs.read(Gpr::Rax), 0x1234_5678);
    }

    #[test]
    fn writing_8_bits_preserves_upper_bits() {
        let mut regs = RegisterFile::default();
        regs.write_width(Gpr::Rax, Width::U64, 0xfeed_face_cafe_beef);
        regs.write_width(Gpr::Rax, Width::U8, 0x11);
        assert_eq!(regs.read(Gpr::Rax), 0xfeed_face_cafe_be11);
    }

    #[test]
    fn rex_extension_selects_high_registers() {
        assert_eq!(Gpr::from_low3(0, false), Gpr::Rax);
        assert_eq!(Gpr::from_low3(0, true), Gpr::R8);
        assert_eq!(Gpr::from_low3(7, true), Gpr::R15);
    }
}
