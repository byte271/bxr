#![forbid(unsafe_code)]

pub const PAGE_SIZE: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryError {
    SizeMustBePageAligned,
    OutOfBounds { addr: u64, len: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalMemory {
    bytes: Vec<u8>,
    dirty_pages: Vec<bool>,
    executable_pages: Vec<bool>,
    page_generations: Vec<u32>,
}

impl PhysicalMemory {
    pub fn new(size: usize) -> Result<Self, MemoryError> {
        if !size.is_multiple_of(PAGE_SIZE) {
            return Err(MemoryError::SizeMustBePageAligned);
        }
        let pages = size / PAGE_SIZE;
        Ok(Self {
            bytes: vec![0; size],
            dirty_pages: vec![false; pages],
            executable_pages: vec![false; pages],
            page_generations: vec![0; pages],
        })
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn read(&self, addr: u64, out: &mut [u8]) -> Result<(), MemoryError> {
        let range = self.checked_range(addr, out.len())?;
        out.copy_from_slice(&self.bytes[range]);
        Ok(())
    }

    pub fn write(&mut self, addr: u64, input: &[u8]) -> Result<(), MemoryError> {
        let range = self.checked_range(addr, input.len())?;
        self.bytes[range.clone()].copy_from_slice(input);
        self.mark_dirty_range(range.start, input.len());
        Ok(())
    }

    pub fn read_u64_le(&self, addr: u64) -> Result<u64, MemoryError> {
        let mut bytes = [0; 8];
        self.read(addr, &mut bytes)?;
        Ok(u64::from_le_bytes(bytes))
    }

    pub fn read_u8(&self, addr: u64) -> Result<u8, MemoryError> {
        let mut byte = [0; 1];
        self.read(addr, &mut byte)?;
        Ok(byte[0])
    }

    pub fn write_u64_le(&mut self, addr: u64, value: u64) -> Result<(), MemoryError> {
        self.write(addr, &value.to_le_bytes())
    }

    pub fn write_u8(&mut self, addr: u64, value: u8) -> Result<(), MemoryError> {
        self.write(addr, &[value])
    }

    pub fn mark_executable(&mut self, page_index: usize) {
        if let Some(page) = self.executable_pages.get_mut(page_index) {
            *page = true;
        }
    }

    pub fn mark_executable_range(&mut self, addr: u64, len: usize) -> Result<(), MemoryError> {
        let range = self.checked_range(addr, len)?;
        if len == 0 {
            return Ok(());
        }

        let first_page = range.start / PAGE_SIZE;
        let last_page = (range.end - 1) / PAGE_SIZE;
        for page in first_page..=last_page {
            self.mark_executable(page);
        }
        Ok(())
    }

    pub fn page_index_for_addr(&self, addr: u64) -> Option<usize> {
        let addr = usize::try_from(addr).ok()?;
        (addr < self.bytes.len()).then_some(addr / PAGE_SIZE)
    }

    pub fn page_generation(&self, page_index: usize) -> Option<u32> {
        self.page_generations.get(page_index).copied()
    }

    pub fn dirty_pages(&self) -> impl Iterator<Item = usize> + '_ {
        self.dirty_pages
            .iter()
            .enumerate()
            .filter_map(|(index, dirty)| dirty.then_some(index))
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_pages.fill(false);
    }

    fn checked_range(&self, addr: u64, len: usize) -> Result<std::ops::Range<usize>, MemoryError> {
        let start = usize::try_from(addr).map_err(|_| MemoryError::OutOfBounds { addr, len })?;
        let end = start
            .checked_add(len)
            .ok_or(MemoryError::OutOfBounds { addr, len })?;
        if end > self.bytes.len() {
            return Err(MemoryError::OutOfBounds { addr, len });
        }
        Ok(start..end)
    }

    fn mark_dirty_range(&mut self, start: usize, len: usize) {
        if len == 0 {
            return;
        }

        let first_page = start / PAGE_SIZE;
        let last_page = (start + len - 1) / PAGE_SIZE;
        for page in first_page..=last_page {
            self.dirty_pages[page] = true;
            if self.executable_pages[page] {
                self.page_generations[page] = self.page_generations[page].wrapping_add(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unaligned_size() {
        assert_eq!(
            PhysicalMemory::new(PAGE_SIZE + 1),
            Err(MemoryError::SizeMustBePageAligned)
        );
    }

    #[test]
    fn write_marks_dirty_pages() {
        let mut memory = PhysicalMemory::new(PAGE_SIZE * 2).unwrap();
        memory.write((PAGE_SIZE - 2) as u64, &[1, 2, 3, 4]).unwrap();
        assert_eq!(memory.dirty_pages().collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn executable_write_advances_generation() {
        let mut memory = PhysicalMemory::new(PAGE_SIZE).unwrap();
        memory.mark_executable(0);
        assert_eq!(memory.page_generation(0), Some(0));
        memory.write(0, &[0x90]).unwrap();
        assert_eq!(memory.page_generation(0), Some(1));
    }

    #[test]
    fn reports_page_index_for_in_bounds_address() {
        let memory = PhysicalMemory::new(PAGE_SIZE * 2).unwrap();

        assert_eq!(memory.page_index_for_addr(0), Some(0));
        assert_eq!(memory.page_index_for_addr(PAGE_SIZE as u64), Some(1));
        assert_eq!(memory.page_index_for_addr((PAGE_SIZE * 2) as u64), None);
    }

    #[test]
    fn executable_range_marks_cross_page_code() {
        let mut memory = PhysicalMemory::new(PAGE_SIZE * 2).unwrap();
        memory
            .mark_executable_range((PAGE_SIZE - 1) as u64, 2)
            .unwrap();

        memory.write((PAGE_SIZE - 1) as u64, &[0x90, 0x90]).unwrap();

        assert_eq!(memory.page_generation(0), Some(1));
        assert_eq!(memory.page_generation(1), Some(1));
    }
}
