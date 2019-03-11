use crate::Starstruck;
use failure::Error;
use futures::Future;
use futures::IntoFuture;
use crate::context::Context;
use std::sync::Arc;
use crate::setup_context::SetupContext;
use crate::allocator::GpuAllocator;
use crate::allocator::DefaultGpuAllocator;
use crate::starstruck::State;
use crate::allocator::DefaultChunk;

/// The main way to construct a starstruck instance
///
/// # Examples
///
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use starstruck::StarstruckBuilder;
///
/// let starstruck = StarstruckBuilder::new().init()?;
/// # Ok(())
/// # }
/// ```
#[allow(clippy::type_complexity)]
pub struct StarstruckBuilder<
    S: State,
    R,
    A: GpuAllocator<backend::Backend, backend::Device>,
> {
    title: String,
    setup_callback: Box<(FnMut(Arc<SetupContext<A>>) -> R + Send)>,
    render_callback: Box<FnMut(
        (
            &mut S,
            &mut Context<A, backend::Backend, backend::Device, backend::Instance>,
        ),
    ) -> Result<(), Error>>,
    allocator: A
}


impl StarstruckBuilder<(), Result<(), Error>, DefaultGpuAllocator<DefaultChunk<backend::Backend, backend::Device>, backend::Backend, backend::Device>> {
    pub fn new() -> Self {
        Self {
            title: "Starstruck".to_string(),
            setup_callback: Box::new(|_| Ok(())),
            render_callback: Box::new(|_| Ok(())),
            allocator: DefaultGpuAllocator::new()
        }
    }
}

impl<
    S: State,
    F: Future<Item=S, Error=Error> + Send + 'static,
    R: IntoFuture<Future=F, Item=S, Error=Error> + 'static
> StarstruckBuilder<S, R, DefaultGpuAllocator<DefaultChunk<backend::Backend, backend::Device>, backend::Backend, backend::Device>> {

    pub fn new_with_setup<T: 'static + FnMut(Arc<SetupContext>) -> R + Send>(setup_callback: T) -> Self {
        Self {
            title: "Starstruck".to_string(),
            setup_callback: Box::new(setup_callback),
            render_callback: Box::new(|_| Ok(())),
            allocator: DefaultGpuAllocator::new()
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }
}

impl<
    S: State,
    F: Future<Item=S, Error=Error> + Send + 'static,
    R: IntoFuture<Future=F, Item=S, Error=Error> + 'static,
    A: GpuAllocator<backend::Backend, backend::Device>
> StarstruckBuilder<S, R, A> {

    pub fn with_render_callback<T: 'static + FnMut(
        (
            &mut S,
            &mut Context<A, backend::Backend, backend::Device, backend::Instance>,
        ),
    ) -> Result<(), Error>>(mut self, callback: T) -> Self {
        self.render_callback = Box::new(callback);
        self
    }

    pub fn init(self) -> Result<Starstruck<S, A>, Error> {
        Starstruck::init(&self.title, self.setup_callback, self.render_callback, self.allocator)
    }
}

impl Default for StarstruckBuilder<(), Result<(), Error>, DefaultGpuAllocator<DefaultChunk<backend::Backend, backend::Device>, backend::Backend, backend::Device>> {
    fn default() -> Self {
        Self::new()
    }
}