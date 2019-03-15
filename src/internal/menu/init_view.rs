use crate::context::Context;
use crate::menu::View;
use failure::Error;
use crate::allocator::GpuAllocator;
use crate::menu::Image;
use std::sync::Arc;
use crate::setup_context::SetupContext;
use futures::Future;
use crate::menu::Component;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;

pub struct InitView<
    A: GpuAllocator<B, D>,
    B: Backend,
    D: Device<B>,
    I: Instance<Backend = B>,
> {
    image: Image<A, B, D, I>
}

impl<
    A: GpuAllocator<B, D>,
    B: Backend,
    D: Device<B>,
    I: Instance<Backend = B>,
> InitView<A, B, D, I> {
    pub fn new(setup: Arc<SetupContext<A, B, D, I>>) -> impl Future<Item=Self, Error=Error> {
        let image_future = Image::new(setup, include_bytes!("star.png"));
        image_future.map(|image|{
            Self {
                image
            }
        })
    }
}

impl<
    A: GpuAllocator<B, D>,
    B: Backend,
    D: Device<B>,
    I: Instance<Backend = B>
> View<A, B, D, I> for InitView<A, B, D, I> {
    fn covers_screen(&self) -> bool {
        true
    }
}

impl<
    A: GpuAllocator<B, D>,
    B: Backend,
    D: Device<B>,
    I: Instance<Backend = B>
> Component<A, B, D, I> for InitView<A, B, D, I> {
    fn resize(&mut self, _size: (u32, u32)) {
        unimplemented!()
    }

    fn draw(&self, context: &mut Context<A, B, D, I>) -> Result<(), Error> {
        self.image.draw(context)?;
        Ok(())
    }
}
