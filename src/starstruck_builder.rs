use crate::Starstruck;
use failure::Error;
use crate::callbacks::State;
use futures::Future;
use futures::IntoFuture;
use crate::context::Context;
use std::sync::Arc;
use crate::setup_context::SetupContext;

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
    R
> {
    title: String,
    setup_callback: Box<(FnMut(Arc<SetupContext>) -> R + Send)>,
    render_callback: Box<FnMut(
        (
            &mut S,
            &mut Context<backend::Backend, backend::Device, backend::Instance>,
        ),
    ) -> Result<(), Error>>
}


impl StarstruckBuilder<(), Result<(), Error>> {
    pub fn new() -> Self {
        Self {
            title: "Starstruck".to_string(),
            setup_callback: Box::new(|_| Ok(())),
            render_callback: Box::new(|_| Ok(()))
        }
    }
}

impl<
    S: State,
    F: Future<Item=S, Error=Error> + Send + 'static,
    R: IntoFuture<Future=F, Item=S, Error=Error> + 'static
> StarstruckBuilder<S, R> {

    pub fn new_with_setup<T: 'static + FnMut(Arc<SetupContext>) -> R + Send>(setup_callback: T) -> Self {
        Self {
            title: "Starstruck".to_string(),
            setup_callback: Box::new(setup_callback),
            render_callback: Box::new(|_| Ok(()))
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn with_render_callback<T: 'static + FnMut(
        (
            &mut S,
            &mut Context<backend::Backend, backend::Device, backend::Instance>,
        ),
    ) -> Result<(), Error>>(mut self, callback: T) -> Self {
        self.render_callback = Box::new(callback);
        self
    }

    pub fn init(self) -> Result<Starstruck<S, backend::Backend, backend::Device, backend::Instance>, Error> {
        Starstruck::init(&self.title, self.setup_callback, self.render_callback)
    }
}

impl Default for StarstruckBuilder<(), Result<(), Error>> {
    fn default() -> Self {
        Self::new()
    }
}