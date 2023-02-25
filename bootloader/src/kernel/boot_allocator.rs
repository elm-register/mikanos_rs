use uefi::prelude::Boot;
use uefi::table::boot::{AllocateType, MemoryType};
use uefi::table::SystemTable;

use libs::error::{LibError, LibResult};
use libs::kernel::loaders::Allocatable;

pub struct BootAllocator<'a>(&'a mut SystemTable<Boot>);

impl<'a> BootAllocator<'a> {
    pub fn new(system_table: &'a mut SystemTable<Boot>) -> Self {
        Self(system_table)
    }
}

impl Allocatable for BootAllocator<'_> {
    fn copy_mem(&self, dest: *mut u8, src: *const u8, size: usize) {
        unsafe {
            self.0.boot_services().memmove(dest, src, size);
        }
    }

    fn set_mem(&mut self, buff: *mut u8, size: usize, value: u8) {
        unsafe {
            self.0.boot_services().set_mem(buff, size, value);
        };
    }

    fn allocate_pool(&self, size: usize) {
        self.0
            .boot_services()
            .allocate_pool(MemoryType::LOADER_DATA, size)
            .unwrap();
    }

    fn allocate_pages(&mut self, phys_addr: u64, count: usize) -> LibResult {
        self.0
            .boot_services()
            .allocate_pages(
                AllocateType::Address(phys_addr),
                MemoryType::LOADER_DATA,
                count,
            ).map_err(|_| LibError::FailedToAllocatePages(phys_addr))?;
        Ok(())
    }
}
