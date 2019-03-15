use crate::menu::Component;
use crate::allocator::GpuAllocator;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;
use crate::context::Context;
use failure::Error;

pub struct Button {}

impl Button {}

impl<
    A: GpuAllocator<B, D>,
    B: Backend,
    D: Device<B>,
    I: Instance<Backend = B>,
> Component<A, B, D, I> for Button {
    fn resize(&mut self, _size: (u32, u32)) {
        unimplemented!()
    }

    fn draw(&self, _context: &mut Context<A, B, D, I>) -> Result<(), Error> {
        unimplemented!()
    }
}
