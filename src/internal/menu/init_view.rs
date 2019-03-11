use crate::context::Context;
use crate::menu::View;
use failure::Error;
use crate::allocator::GpuAllocator;

pub struct InitView {}

impl InitView {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {})
    }
}

impl<A: GpuAllocator> View<A> for InitView {
    fn draw(&self, _context: &Context<A>) -> Result<(), Error> {
        Ok(())
    }

    fn covers_screen(&self) -> bool {
        true
    }
}
