use crate::context::Context;
use failure::Error;
use crate::allocator::GpuAllocator;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;


pub trait Component<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> {
    fn resize(&mut self, size: (u32, u32));
    fn draw(&self, context: &mut Context<A, B, D, I>) -> Result<(), Error>;
}
