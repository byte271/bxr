use crate::Width;

const RESERVED_ALWAYS_ONE: u64 = 1 << 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Flag {
    Carry,
    Parity,
    AuxiliaryCarry,
    Zero,
    Sign,
    Trap,
    InterruptEnable,
    Direction,
    Overflow,
}

impl Flag {
    const fn bit(self) -> u8 {
        match self {
            Self::Carry => 0,
            Self::Parity => 2,
            Self::AuxiliaryCarry => 4,
            Self::Zero => 6,
            Self::Sign => 7,
            Self::Trap => 8,
            Self::InterruptEnable => 9,
            Self::Direction => 10,
            Self::Overflow => 11,
        }
    }

    const fn mask(self) -> u64 {
        1_u64 << self.bit()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RFlags {
    bits: u64,
}

impl Default for RFlags {
    fn default() -> Self {
        Self::new()
    }
}

impl RFlags {
    pub const fn new() -> Self {
        Self {
            bits: RESERVED_ALWAYS_ONE,
        }
    }

    pub const fn from_bits(bits: u64) -> Self {
        Self {
            bits: bits | RESERVED_ALWAYS_ONE,
        }
    }

    pub const fn bits(self) -> u64 {
        self.bits | RESERVED_ALWAYS_ONE
    }

    pub fn get(self, flag: Flag) -> bool {
        (self.bits() & flag.mask()) != 0
    }

    pub fn set(&mut self, flag: Flag, enabled: bool) {
        if enabled {
            self.bits |= flag.mask();
        } else {
            self.bits &= !flag.mask();
        }
        self.bits |= RESERVED_ALWAYS_ONE;
    }

    pub fn update_logic_result(&mut self, width: Width, result: u64) {
        let result = width.truncate(result);
        self.set(Flag::Carry, false);
        self.set(Flag::Overflow, false);
        self.set(Flag::Sign, (result & width.sign_bit()) != 0);
        self.set(Flag::Zero, result == 0);
        self.set(Flag::Parity, even_low_byte_parity(result));
    }

    pub fn update_add(&mut self, width: Width, lhs: u64, rhs: u64, result: u64) {
        let mask = width.mask();
        let sign = width.sign_bit();
        let lhs = lhs & mask;
        let rhs = rhs & mask;
        let result = result & mask;

        self.set(Flag::Carry, lhs > mask.wrapping_sub(rhs));
        self.set(Flag::AuxiliaryCarry, ((lhs ^ rhs ^ result) & 0x10) != 0);
        self.set(
            Flag::Overflow,
            ((!(lhs ^ rhs) & (lhs ^ result)) & sign) != 0,
        );
        self.set(Flag::Sign, (result & sign) != 0);
        self.set(Flag::Zero, result == 0);
        self.set(Flag::Parity, even_low_byte_parity(result));
    }

    pub fn update_sub(&mut self, width: Width, lhs: u64, rhs: u64, result: u64) {
        let mask = width.mask();
        let sign = width.sign_bit();
        let lhs = lhs & mask;
        let rhs = rhs & mask;
        let result = result & mask;

        self.set(Flag::Carry, lhs < rhs);
        self.set(Flag::AuxiliaryCarry, ((lhs ^ rhs ^ result) & 0x10) != 0);
        self.set(Flag::Overflow, (((lhs ^ rhs) & (lhs ^ result)) & sign) != 0);
        self.set(Flag::Sign, (result & sign) != 0);
        self.set(Flag::Zero, result == 0);
        self.set(Flag::Parity, even_low_byte_parity(result));
    }
}

fn even_low_byte_parity(value: u64) -> bool {
    (value as u8).count_ones().is_multiple_of(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_bit_stays_set() {
        let mut flags = RFlags::from_bits(0);
        flags.set(Flag::Carry, true);
        flags.set(Flag::Carry, false);
        assert_eq!(flags.bits() & RESERVED_ALWAYS_ONE, RESERVED_ALWAYS_ONE);
    }

    #[test]
    fn add_sets_carry_and_zero() {
        let mut flags = RFlags::new();
        flags.update_add(Width::U8, 0xff, 0x01, 0x00);
        assert!(flags.get(Flag::Carry));
        assert!(flags.get(Flag::Zero));
        assert!(!flags.get(Flag::Sign));
    }

    #[test]
    fn add_sets_signed_overflow() {
        let mut flags = RFlags::new();
        flags.update_add(Width::U8, 0x7f, 0x01, 0x80);
        assert!(flags.get(Flag::Overflow));
        assert!(flags.get(Flag::Sign));
    }

    #[test]
    fn sub_sets_borrow() {
        let mut flags = RFlags::new();
        flags.update_sub(Width::U16, 0x0000, 0x0001, 0xffff);
        assert!(flags.get(Flag::Carry));
        assert!(flags.get(Flag::Sign));
    }
}
