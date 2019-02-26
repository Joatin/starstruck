use gfx_hal::Backend;
use std::sync::Arc;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use gfx_hal::device::Device;

pub struct FutureFence {
    fence: ManuallyDrop<<backend::Backend as Backend>::Fence>,
    device: Arc<backend::Device>
}

impl FutureFence {
    pub fn new(fence: <backend::Backend as Backend>::Fence, device: Arc<backend::Device>) -> Self {
        Self {
            fence: ManuallyDrop::new(fence),
            device
        }
    }
}

impl Drop for FutureFence {
    fn drop(&mut self) {
        info!("Dropping fence");
        use core::ptr::read;
        unsafe {
            self.device.destroy_fence(ManuallyDrop::into_inner( read(&self.fence)));
        }
    }
}

pub trait FenceExt {
    fn into_promise(self, device: Arc<backend::Device>) -> FutureFence;
}

impl FenceExt for <backend::Backend as Backend>::Fence {
    fn into_promise(self, device: Arc<backend::Device>) -> FutureFence {
        FutureFence::new(self, device)
    }
}

impl Deref for FutureFence {
    type Target = <backend::Backend as Backend>::Fence;

    fn deref(&self) -> &Self::Target {
        &self.fence
    }
}