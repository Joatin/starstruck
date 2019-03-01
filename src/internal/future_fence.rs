use gfx_hal::device::Device;
use gfx_hal::Backend;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::sync::Arc;

pub struct FutureFence<B: Backend, D: Device<B>> {
    fence: ManuallyDrop<B::Fence>,
    device: Arc<D>,
}

impl<B: Backend, D: Device<B>> FutureFence<B, D> {
    pub fn new(fence: B::Fence, device: Arc<D>) -> Self {
        Self {
            fence: ManuallyDrop::new(fence),
            device,
        }
    }
}

impl<B: Backend, D: Device<B>> Drop for FutureFence<B, D> {
    fn drop(&mut self) {
        use core::ptr::read;
        unsafe {
            self.device
                .destroy_fence(ManuallyDrop::into_inner(read(&self.fence)));
        }
    }
}

pub trait FenceExt<B: Backend, D: Device<B>> {
    fn into_promise(self, device: Arc<D>) -> FutureFence<B, D>;
}

impl<B: Backend, D: Device<B>> FenceExt<B, D> for B::Fence {
    fn into_promise(self, device: Arc<D>) -> FutureFence<B, D> {
        FutureFence::new(self, device)
    }
}

impl<B: Backend, D: Device<B>> Deref for FutureFence<B, D> {
    type Target = B::Fence;

    fn deref(&self) -> &Self::Target {
        &self.fence
    }
}
