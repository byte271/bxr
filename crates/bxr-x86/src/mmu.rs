use crate::system::ControlRegisters;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessType {
    Read,
    Write,
    Execute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivilegeLevel {
    Supervisor,
    User,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranslateRequest {
    pub virtual_addr: u64,
    pub access: AccessType,
    pub privilege: PrivilegeLevel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Translation {
    pub physical_addr: u64,
    pub page_size: PageSize,
    pub writable: bool,
    pub user: bool,
    pub executable: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageSize {
    Size4K,
    Size2M,
    Size1G,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageFault {
    pub addr: u64,
    pub error_code: u32,
    pub reason: PageFaultReason,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageFaultReason {
    NotPresent,
    WriteToReadOnly,
    UserToSupervisor,
    ExecuteDisabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MmuError {
    NonCanonical { addr: u64 },
    PageFault(PageFault),
    PhysicalRead { addr: u64 },
    Unsupported(&'static str),
}

pub trait PageTableMemory {
    fn read_u64_phys(&self, addr: u64) -> Result<u64, MmuError>;
}

pub fn translate(
    controls: ControlRegisters,
    memory: &impl PageTableMemory,
    request: TranslateRequest,
) -> Result<Translation, MmuError> {
    if !controls.paging_enabled() {
        return Ok(Translation {
            physical_addr: request.virtual_addr,
            page_size: PageSize::Size4K,
            writable: true,
            user: true,
            executable: true,
        });
    }

    if !controls.pae_enabled() {
        return Err(MmuError::Unsupported("x86-64 paging requires CR4.PAE"));
    }
    if !controls.long_mode_enabled() {
        return Err(MmuError::Unsupported(
            "only long-mode page walks are implemented",
        ));
    }
    if !is_canonical_48(request.virtual_addr) {
        return Err(MmuError::NonCanonical {
            addr: request.virtual_addr,
        });
    }

    let indexes = [
        ((request.virtual_addr >> 39) & 0x1ff) as u16,
        ((request.virtual_addr >> 30) & 0x1ff) as u16,
        ((request.virtual_addr >> 21) & 0x1ff) as u16,
        ((request.virtual_addr >> 12) & 0x1ff) as u16,
    ];
    let mut table = controls.cr3_base();
    let mut rights = AccessRights {
        writable: true,
        user: true,
        executable: true,
    };

    let pml4e = read_entry(memory, table, indexes[0])?;
    apply_entry_rights(controls, request, pml4e, &mut rights)?;

    table = entry_addr_4k(pml4e);
    let pdpte = read_entry(memory, table, indexes[1])?;
    apply_entry_rights(controls, request, pdpte, &mut rights)?;
    if entry_large_page(pdpte) {
        return finalize_translation(
            request,
            pdpte & 0x000f_ffff_c000_0000,
            request.virtual_addr & 0x3fff_ffff,
            PageSize::Size1G,
            rights,
        );
    }

    table = entry_addr_4k(pdpte);
    let pde = read_entry(memory, table, indexes[2])?;
    apply_entry_rights(controls, request, pde, &mut rights)?;
    if entry_large_page(pde) {
        return finalize_translation(
            request,
            pde & 0x000f_ffff_ffe0_0000,
            request.virtual_addr & 0x1f_ffff,
            PageSize::Size2M,
            rights,
        );
    }

    table = entry_addr_4k(pde);
    let pte = read_entry(memory, table, indexes[3])?;
    apply_entry_rights(controls, request, pte, &mut rights)?;

    finalize_translation(
        request,
        entry_addr_4k(pte),
        request.virtual_addr & 0xfff,
        PageSize::Size4K,
        rights,
    )
}

#[derive(Clone, Copy)]
struct AccessRights {
    writable: bool,
    user: bool,
    executable: bool,
}

fn apply_entry_rights(
    controls: ControlRegisters,
    request: TranslateRequest,
    entry: u64,
    rights: &mut AccessRights,
) -> Result<(), MmuError> {
    check_present(request, entry)?;
    rights.writable &= entry_writable(entry);
    rights.user &= entry_user(entry);
    rights.executable &= entry_executable(controls, entry);
    Ok(())
}

fn finalize_translation(
    request: TranslateRequest,
    base: u64,
    offset: u64,
    page_size: PageSize,
    rights: AccessRights,
) -> Result<Translation, MmuError> {
    check_rights(request, rights)?;
    Ok(Translation {
        physical_addr: base | offset,
        page_size,
        writable: rights.writable,
        user: rights.user,
        executable: rights.executable,
    })
}

fn read_entry(memory: &impl PageTableMemory, table: u64, index: u16) -> Result<u64, MmuError> {
    let addr = table + u64::from(index) * 8;
    memory.read_u64_phys(addr)
}

fn check_present(request: TranslateRequest, entry: u64) -> Result<(), MmuError> {
    if (entry & 1) == 0 {
        return Err(page_fault(request, false, PageFaultReason::NotPresent));
    }
    Ok(())
}

fn check_rights(request: TranslateRequest, rights: AccessRights) -> Result<(), MmuError> {
    if request.access == AccessType::Write && !rights.writable {
        return Err(page_fault(request, true, PageFaultReason::WriteToReadOnly));
    }
    if request.privilege == PrivilegeLevel::User && !rights.user {
        return Err(page_fault(request, true, PageFaultReason::UserToSupervisor));
    }
    if request.access == AccessType::Execute && !rights.executable {
        return Err(page_fault(request, true, PageFaultReason::ExecuteDisabled));
    }
    Ok(())
}

fn page_fault(request: TranslateRequest, present: bool, reason: PageFaultReason) -> MmuError {
    let mut error_code = 0;
    if present {
        error_code |= 1;
    }
    if request.access == AccessType::Write {
        error_code |= 1 << 1;
    }
    if request.privilege == PrivilegeLevel::User {
        error_code |= 1 << 2;
    }
    if request.access == AccessType::Execute {
        error_code |= 1 << 4;
    }
    MmuError::PageFault(PageFault {
        addr: request.virtual_addr,
        error_code,
        reason,
    })
}

fn entry_addr_4k(entry: u64) -> u64 {
    entry & 0x000f_ffff_ffff_f000
}

fn entry_writable(entry: u64) -> bool {
    (entry & (1 << 1)) != 0
}

fn entry_user(entry: u64) -> bool {
    (entry & (1 << 2)) != 0
}

fn entry_large_page(entry: u64) -> bool {
    (entry & (1 << 7)) != 0
}

fn entry_executable(controls: ControlRegisters, entry: u64) -> bool {
    !controls.nx_enabled() || (entry & (1_u64 << 63)) == 0
}

fn is_canonical_48(addr: u64) -> bool {
    let sign = (addr >> 47) & 1;
    let upper = addr >> 48;
    (sign == 0 && upper == 0) || (sign == 1 && upper == 0xffff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct TestPageTables {
        entries: BTreeMap<u64, u64>,
    }

    impl TestPageTables {
        fn write_entry(&mut self, addr: u64, value: u64) {
            self.entries.insert(addr, value);
        }
    }

    impl PageTableMemory for TestPageTables {
        fn read_u64_phys(&self, addr: u64) -> Result<u64, MmuError> {
            self.entries
                .get(&addr)
                .copied()
                .ok_or(MmuError::PhysicalRead { addr })
        }
    }

    fn paging_controls() -> ControlRegisters {
        ControlRegisters {
            cr0: ControlRegisters::CR0_PG,
            cr3: 0x1000,
            cr4: ControlRegisters::CR4_PAE,
            efer: ControlRegisters::EFER_LME | ControlRegisters::EFER_LMA,
            ..ControlRegisters::default()
        }
    }

    #[test]
    fn no_paging_uses_identity_translation() {
        let memory = TestPageTables::default();
        let translation = translate(
            ControlRegisters::default(),
            &memory,
            TranslateRequest {
                virtual_addr: 0x1234,
                access: AccessType::Execute,
                privilege: PrivilegeLevel::Supervisor,
            },
        )
        .unwrap();

        assert_eq!(translation.physical_addr, 0x1234);
    }

    #[test]
    fn translates_4k_page_in_long_mode() {
        let mut memory = TestPageTables::default();
        memory.write_entry(0x1000, 0x2000 | 0b111);
        memory.write_entry(0x2000, 0x3000 | 0b111);
        memory.write_entry(0x3000, 0x4000 | 0b111);
        memory.write_entry(0x4008, 0x8000 | 0b111);

        let translation = translate(
            paging_controls(),
            &memory,
            TranslateRequest {
                virtual_addr: 0x1000,
                access: AccessType::Read,
                privilege: PrivilegeLevel::User,
            },
        )
        .unwrap();

        assert_eq!(translation.physical_addr, 0x8000);
        assert_eq!(translation.page_size, PageSize::Size4K);
    }

    #[test]
    fn write_to_read_only_page_faults() {
        let mut memory = TestPageTables::default();
        memory.write_entry(0x1000, 0x2000 | 0b111);
        memory.write_entry(0x2000, 0x3000 | 0b111);
        memory.write_entry(0x3000, 0x4000 | 0b111);
        memory.write_entry(0x4008, 0x8000 | 0b101);

        let err = translate(
            paging_controls(),
            &memory,
            TranslateRequest {
                virtual_addr: 0x1000,
                access: AccessType::Write,
                privilege: PrivilegeLevel::User,
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            page_fault(
                TranslateRequest {
                    virtual_addr: 0x1000,
                    access: AccessType::Write,
                    privilege: PrivilegeLevel::User,
                },
                true,
                PageFaultReason::WriteToReadOnly
            )
        );
    }

    #[test]
    fn nx_page_execute_faults_when_enabled() {
        let mut memory = TestPageTables::default();
        memory.write_entry(0x1000, 0x2000 | 0b111);
        memory.write_entry(0x2000, 0x3000 | 0b111);
        memory.write_entry(0x3000, 0x4000 | 0b111);
        memory.write_entry(0x4008, (1_u64 << 63) | 0x8000 | 0b111);
        let mut controls = paging_controls();
        controls.efer |= ControlRegisters::EFER_NXE;

        let err = translate(
            controls,
            &memory,
            TranslateRequest {
                virtual_addr: 0x1000,
                access: AccessType::Execute,
                privilege: PrivilegeLevel::User,
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            page_fault(
                TranslateRequest {
                    virtual_addr: 0x1000,
                    access: AccessType::Execute,
                    privilege: PrivilegeLevel::User,
                },
                true,
                PageFaultReason::ExecuteDisabled
            )
        );
    }
}
