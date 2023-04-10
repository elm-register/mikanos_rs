#![feature(pointer_byte_offsets)]
#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![feature(strict_provenance)]
#![test_runner(test_runner::my_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]
extern crate alloc;

use core::alloc::Layout;
use core::num::NonZeroUsize;
use core::panic::PanicInfo;

use uefi::table::boot::{MemoryMapIter, MemoryType};
use volatile::Volatile;
use x86_64::instructions::interrupts::{enable, enable_and_hlt};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use allocate::init_alloc;
use common_lib::frame_buffer::FrameBufferConfig;
use common_lib::queue::queueing::Queueing;
use common_lib::queue::vector_queue::VectorQueue;
use common_lib::vector::Vector2D;
use kernel_lib::{println, serial_println};
use kernel_lib::error::KernelResult;
use kernel_lib::gop::console::{
    CONSOLE_BACKGROUND_COLOR, draw_cursor, erase_cursor, fill_rect_using_global, init_console,
};
use kernel_lib::gop::pixel::pixel_color::PixelColor;
use kernel_lib::interrupt::interrupt_descriptor::init_gdt;
use pci::class_driver::mouse::mouse_driver_factory::MouseDriverFactory;
use pci::class_driver::mouse::MouseButton;
use pci::configuration_space::common_header::class_code::ClassCode;
use pci::configuration_space::common_header::sub_class::Subclass;
use pci::configuration_space::device::header_type::general_header::GeneralHeader;
use pci::configuration_space::io::io_memory_accessible::real_memory_accessor::RealIoMemoryAccessor;
use pci::configuration_space::msi::InterruptCapabilityRegisterIter;
use pci::configuration_space::msi::msi_capability_register::structs::message_data::delivery_mode::DeliveryMode;
use pci::configuration_space::msi::msi_capability_register::structs::message_data::interrupt_vector::InterruptVector;
use pci::configuration_space::msi::msi_capability_register::structs::message_data::trigger_mode::TriggerMode;
use pci::error::PciResult;
use pci::pci_device_searcher::PciDeviceSearcher;
use pci::xhc::allocator::mikanos_pci_memory_allocator::MikanOSPciMemoryAllocator;
use pci::xhc::registers::external::External;
use pci::xhc::registers::internal::memory_mapped_addr::MemoryMappedAddr;
use pci::xhc::XhcController;

static mut QUEUE: VectorQueue<u32> = VectorQueue::new();
static mut IDTA: InterruptDescriptorTable = InterruptDescriptorTable::new();
//
// static mut XHC: Once<
//     XhcController<
//         External<IdentityMapper>,
//         DeviceMap<External<IdentityMapper>, MikanOSPciMemoryAllocator>,
//         MikanOSPciMemoryAllocator,
//     >,
// > = Once::new();

// 読み書き可能でヒープに確保
pub fn init_idt() {
    unsafe {
        // IDT.breakpoint
        //     .set_handler_fn(double_fault_handler);
        let a = IDTA[0x40].set_handler_fn(IntHandlerXHCI);

        IDTA.load();
    }
}


pub const PIC_1_OFFSET: u8 = 0x40;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// new
extern "x86-interrupt" fn IntHandlerXHCI(stack_frame: InterruptStackFrame) {
    unsafe {
        QUEUE.enqueue(32);
        println!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
        // ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET).notify_end_of_interrupt(0x40);
        //
        #[allow(clippy::unwrap_used)]
        let mut memory = Volatile::new(unsafe {
            (0xfee000b0 as *mut u32)
                .as_mut()
                .unwrap()
        });
        memory.write(0);

        // *end_of_interrupt_register_ptr = 0;
    }
}

pub mod allocate;
mod qemu;
#[cfg(test)]
mod test_runner;
#[cfg(test)]
macros::declaration_volatile_accessible!();
// #[no_mangle]
// pub extern "sysv64" fn kernel_entry_point(
//     frame_buffer_config: &FrameBufferConfig,
//     memory_map: &MemoryMapIter,
// ) {
//     let address = KERNEL_STACK.end_addr();
//
//     unsafe {
//         asm!(
//             "mov rsp, {0}",
//             "call kernel_main",
//
//             in(reg) address,
//             in("rdi") frame_buffer_config,
//             in("esi") memory_map,
//             clobber_abi("sysv64")
//         )
//     }
// }
extern "C" {
    fn LoadIDT(limit: u16, offset: u64);
    fn GetCS() -> u16;
}

#[no_mangle]
pub extern "sysv64" fn kernel_main(
    frame_buffer_config: &FrameBufferConfig,
    _memory_map: &MemoryMapIter,
) {
    // unsafe { setup_segments() };

    init_alloc();

    init_console(*frame_buffer_config);

    init_idt();
    #[cfg(test)]
    test_main();
    serial_println!("Hello Serial Port!");
    println!("Hello Kernel!");

    fill_background(CONSOLE_BACKGROUND_COLOR, frame_buffer_config).unwrap();
    fill_bottom_bar(PixelColor::new(0, 0, 0xFF), frame_buffer_config).unwrap();
    let addr = unsafe { ((IntHandlerXHCI) as *const ()) as u64 };

    // unsafe {
    //     set_idt_entry(&mut IDT[0x40], make_idt_attr(14, 0, true, 0), addr,
    // GetCS());     LoadIDT(
    //         (core::mem::size_of::<InterruptDescriptor>() * (IDT.len() - 1)) as
    // u16,         IDT.as_ptr() as u64,
    //     );
    // }

    enable_msi().unwrap();
    enable();
    let external = External::new(mmio_base_addr(), IdentityMapper());
    let mut xhc_controller = XhcController::new(
        external,
        MikanOSPciMemoryAllocator::new(),
        MouseDriverFactory::subscriber(on_mouse_move),
    )
    .unwrap();

    // unsafe {
    //     XHC.call_once(|| xhc_controller);
    //     XHC.get_mut()
    //         .unwrap()
    //         .reset_port();
    // }

    xhc_controller.reset_port();
    serial_println!("{:?}", RealIoMemoryAccessor::new());

    // xhc_controller
    //     .start_event_pooling()
    //     .unwrap();
    // let queue_waiter = unsafe { InterruptQueueWaiter::new(&mut QUEUE) };
    // queue_waiter.for_each(|event| {
    //     serial_println!("Interrupt!");
    //     xhc_controller.process_all_events();
    // });
    loop {
        serial_println!("{}", unsafe { QUEUE.count() });
        let a = unsafe { QUEUE.count() == 0 };
        if a {
            enable_and_hlt();
            continue;
        }
        let a = unsafe { QUEUE.dequeue().unwrap() };
        xhc_controller.process_all_events();
    }

    common_lib::assembly::hlt_forever();
}

fn on_mouse_move(
    prev_cursor: Vector2D<usize>,
    current_cursor: Vector2D<usize>,
    button: Option<MouseButton>,
) -> Result<(), ()> {
    let color = button
        .map(|b| match b {
            MouseButton::Button1 => PixelColor::yellow(),
            MouseButton::Button2 => PixelColor::new(0x13, 0xA9, 0xDB),
            MouseButton::Button3 => PixelColor::new(0x35, 0xFA, 0x66),
            _ => PixelColor::white(),
        })
        .unwrap_or(PixelColor::white());

    erase_cursor(prev_cursor).map_err(|_| ())?;
    draw_cursor(current_cursor, color).map_err(|_| ())
}

#[derive(Clone, Debug)]
struct IdentityMapper();

impl xhci::accessor::Mapper for IdentityMapper {
    unsafe fn map(&mut self, phys_start: usize, _bytes: usize) -> NonZeroUsize {
        NonZeroUsize::new_unchecked(phys_start)
    }

    fn unmap(&mut self, _virtual_start: usize, _bytes: usize) {}
}

#[allow(dead_code)]
fn is_available(memory_type: MemoryType) -> bool {
    match memory_type {
        MemoryType::BOOT_SERVICES_CODE
        | MemoryType::BOOT_SERVICES_DATA
        | MemoryType::MMIO
        | MemoryType::MMIO_PORT_SPACE
        | MemoryType::CONVENTIONAL => true,
        _ => false,
    }
}

#[allow(dead_code)]
fn fill_background(color: PixelColor, config: &FrameBufferConfig) -> KernelResult {
    fill_rect_using_global(
        Vector2D::new(0, 0),
        Vector2D::new(config.horizontal_resolution, config.vertical_resolution),
        color,
    )
}

#[allow(dead_code)]
fn fill_bottom_bar(color: PixelColor, config: &FrameBufferConfig) -> KernelResult {
    let v = config.vertical_resolution;
    let h = config.horizontal_resolution;
    fill_rect_using_global(Vector2D::new(0, v - 50), Vector2D::new(h, v), color)?;
    fill_rect_using_global(
        Vector2D::new(0, v - 50),
        Vector2D::new(50, v),
        PixelColor::new(0x33, 0x33, 0xAA),
    )
}


pub fn first_general_header() -> GeneralHeader {
    PciDeviceSearcher::new()
        .class_code(ClassCode::SerialBus)
        .sub_class(Subclass::Usb)
        .search()
        .unwrap()
        .cast_device()
        .expect_single()
        .unwrap()
        .expect_general()
        .unwrap()
}

fn enable_msi() -> PciResult {
    let io = RealIoMemoryAccessor::new();
    let bsp_local_apic_id: u8 = unsafe { *(0xfee00020 as *mut u32) >> 24 } as u8;

    for mut msi in InterruptCapabilityRegisterIter::new(first_general_header(), io)
        .filter_map(|register| register.ok())
        .filter_map(|register| register.msi())
    {
        msi.enable(
            bsp_local_apic_id,
            TriggerMode::Level,
            InterruptVector::Xhci,
            DeliveryMode::Fixed,
        )?;
        serial_println!("{:?}", msi);
    }
    Ok(())
}

#[allow(dead_code)]
fn mmio_base_addr() -> MemoryMappedAddr {
    first_general_header().mmio_base_addr()
}

/// この関数はパニック時に呼ばれる
#[panic_handler]
#[cfg(not(test))]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    common_lib::assembly::hlt_forever();
}

#[panic_handler]
#[cfg(test)]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[test failed!]");
    serial_println!("{}", info);
    qemu::exit_qemu(qemu::QemuExitCode::Failed);
}

#[alloc_error_handler]
fn on_oom(layout: Layout) -> ! {
    println!("Failed Heap Allocate! {:?}", layout);
    common_lib::assembly::hlt_forever();
}
