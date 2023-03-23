use pci::configuration_space::common_header::class_code::ClassCode;
use pci::configuration_space::common_header::sub_class::Subclass;
use pci::pci_device_searcher::PciDeviceSearcher;
use pci::xhci::registers::capability_registers::capability_length::CapabilityLength;
use pci::xhci::registers::capability_registers::structural_parameters2::StructuralParameters2Offset;
use pci::xhci::registers::memory_mapped_addr::MemoryMappedAddr;
use pci::xhci::registers::operational_registers::command_ring_control_register::CommandRingControlRegisterOffset;
use pci::xhci::registers::operational_registers::config_register::ConfigRegisterOffset;
use pci::xhci::registers::operational_registers::device_context_base_address_array_pointer::DeviceContextBaseAddressArrayPointerOffset;
use pci::xhci::registers::operational_registers::operation_registers_offset::OperationalRegistersOffset;
use pci::xhci::registers::operational_registers::usb_status_register::usb_status_register_offset::UsbStatusRegisterOffset;
use pci::xhci::registers::operational_registers::OperationalRegisters;
use pci::xhci::registers::Registers;

mod registers;

pub(crate) fn mmio_base_addr() -> MemoryMappedAddr {
    let mouse = PciDeviceSearcher::new()
        .class_code(ClassCode::SerialBus)
        .sub_class(Subclass::Usb)
        .search()
        .unwrap()
        .cast_device()
        .expect_single()
        .unwrap()
        .expect_general()
        .unwrap();

    mouse.mmio_base_addr()
}

pub(crate) fn registers() -> Registers {
    Registers::new(mmio_base_addr()).unwrap()
}

pub(crate) fn operational_registers() -> OperationalRegisters {
    OperationalRegisters::new(operation_registers_offset()).unwrap()
}

#[allow(dead_code)]
pub(crate) fn hcs_params2_offset() -> StructuralParameters2Offset {
    StructuralParameters2Offset::new(crate::mmio_base_addr())
}

#[allow(dead_code)]
pub(crate) fn operation_registers_offset() -> OperationalRegistersOffset {
    let mmio_base_addr = mmio_base_addr();
    let cap_length = CapabilityLength::new_check_length(mmio_base_addr).unwrap();
    OperationalRegistersOffset::new(mmio_base_addr, &cap_length)
}

#[allow(dead_code)]
pub(crate) fn config_register_offset() -> ConfigRegisterOffset {
    ConfigRegisterOffset::new(operation_registers_offset())
}

#[allow(dead_code)]
pub(crate) fn usb_status_register_offset() -> UsbStatusRegisterOffset {
    let mmio_addr = mmio_base_addr();
    let cap_length = CapabilityLength::new_check_length(mmio_addr).unwrap();
    UsbStatusRegisterOffset::new(OperationalRegistersOffset::new(mmio_addr, &cap_length))
}

#[allow(dead_code)]
pub(crate) fn dcbaap_offset() -> DeviceContextBaseAddressArrayPointerOffset {
    DeviceContextBaseAddressArrayPointerOffset::new(operation_registers_offset())
}

pub(crate) fn command_ring_control_register_offset() -> CommandRingControlRegisterOffset {
    CommandRingControlRegisterOffset::new(operation_registers_offset())
}
