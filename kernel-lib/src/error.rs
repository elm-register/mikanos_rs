use core::num::TryFromIntError;

use common_lib::math::rectangle::Rectangle;

pub type KernelResult<T = ()> = Result<T, KernelError>;


/// Errors emitted from kernel-lib
#[derive(Debug)]
pub enum KernelError {
    ExceededFrameBufferSize,
    NotSupportCharacter,
    FailedCast,
    NumSizeOver,
    FailedOperateLayer(LayerReason),
    FailedAllocate(AllocateReason),
    TryFromIntError(TryFromIntError),
}


#[derive(Debug)]
pub enum LayerReason {
    FailedInitialize,
    NotExistsKey,
    InvalidCastWindowDrawer,
    WindowSizeOver(Rectangle<usize>),
}


#[derive(Debug)]
pub enum AllocateReason {
    InitializeGlobalAllocator,
    OverFrame {
        max_frame_id: usize,
        frame_id: usize,
    },
    OverAddress {
        address: u64,
    },
}
