use alloc::vec::Vec;

use libs::error::LibResult;
use uefi::proto::media::file::{Directory, File, FileInfo, FileMode, RegularFile};
use uefi_services::println;

use libs::kernel::loaders::elf_loader::ElfLoader;
use libs::kernel::loaders::{Allocatable, KernelLoadable};

use crate::file::open_file;

pub fn execute_kernel(
    root_dir: &mut Directory,
    kernel_file_path: &str,
    allocator: &mut impl Allocatable,
) -> LibResult {
    let mut kernel_file = open_file(root_dir, kernel_file_path, FileMode::Read)
        .map(|file_handle| file_handle.into_regular_file())
        .expect("should open kernel.libs")
        .unwrap();

    let kernel_file_size = get_kernel_file_size(&mut kernel_file) as usize;
    const KERNEL_BASE_ADDR: u64 = 0x100_000u64;
    let entry_point = KERNEL_BASE_ADDR as *mut u8;

    let buff: &mut [u8] =
        unsafe { core::slice::from_raw_parts_mut(entry_point, kernel_file_size as usize) };
    allocator.allocate_pool(kernel_file_size);

    read_kernel_buff(&mut kernel_file, buff);

    let entry_point = ElfLoader::new().load(buff, allocator)?;

    println!("kernel_entry_point_addr = {:#08x}", entry_point);
    entry_point.execute();
    Ok(())
}

fn get_kernel_file_size(kernel_file: &mut RegularFile) -> u64 {
    // カーネルファイルの大きさを知るため、ファイル情報を読み取る
    const FILE_INFO_SIZE: usize = 4000;

    let mut buff = Vec::<u8>::new();
    buff.resize(FILE_INFO_SIZE, 0);
    let info = kernel_file
        .get_info::<FileInfo>(buff.as_mut_slice())
        .expect("should obtain kernel libs info");

    info.file_size()
}

fn read_kernel_buff(kernel_file: &mut RegularFile, buff: &mut [u8]) {
    kernel_file.read(buff).unwrap();
}
