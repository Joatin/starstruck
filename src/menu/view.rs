use crate::allocator::GpuAllocator;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;
use crate::menu::Component;

pub trait View<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>>: Component<A, B, D, I> {
    fn covers_screen(&self) -> bool;
}
