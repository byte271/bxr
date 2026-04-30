#![forbid(unsafe_code)]

use bxr_x86::{Flag, Gpr, RFlags, RegisterFile, Width};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectX64BootState {
    pub entry: u64,
    pub stack_top: u64,
}

pub fn apply_direct_x64_boot_state(
    registers: &mut RegisterFile,
    flags: &mut RFlags,
    boot: DirectX64BootState,
) {
    registers.set_rip(boot.entry);
    registers.write_width(Gpr::Rsp, Width::U64, boot.stack_top);
    flags.set(Flag::InterruptEnable, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_boot_sets_entry_and_stack() {
        let mut registers = RegisterFile::default();
        let mut flags = RFlags::new();
        flags.set(Flag::InterruptEnable, true);

        apply_direct_x64_boot_state(
            &mut registers,
            &mut flags,
            DirectX64BootState {
                entry: 0x100000,
                stack_top: 0x800000,
            },
        );

        assert_eq!(registers.rip(), 0x100000);
        assert_eq!(registers.read(Gpr::Rsp), 0x800000);
        assert!(!flags.get(Flag::InterruptEnable));
    }
}
