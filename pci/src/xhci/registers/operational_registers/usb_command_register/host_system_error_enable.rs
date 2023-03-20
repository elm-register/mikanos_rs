use core::marker::PhantomData;

use macros::VolatileBits;

use crate::xhci::registers::operational_registers::operation_registers_offset::OperationalRegistersOffset;

#[derive(VolatileBits)]
#[offset_bit(3)]
#[bits(1)]
pub struct HostSystemErrorEnable(usize, PhantomData<OperationalRegistersOffset>);
