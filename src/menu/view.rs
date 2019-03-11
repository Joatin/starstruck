use crate::context::Context;
use failure::Error;
use crate::allocator::GpuAllocator;

pub trait View<A: GpuAllocator> {
    fn draw(&self, context: &Context<A>) -> Result<(), Error>;
    fn covers_screen(&self) -> bool;
}
