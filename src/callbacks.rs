use crate::context::Context;
use failure::Error;
use futures::Future;
use crate::setup_context::SetupContext;
use std::sync::Arc;
use futures::IntoFuture;

pub trait State: 'static + Send + Sync {}

impl <T: 'static + Send + Sync> State for T {}

pub trait RenderCallback<S: State>: FnMut(
    (
        &mut S,
        &mut Context<backend::Backend, backend::Device, backend::Instance>,
    ),
) -> Result<(), Error> {}

impl<S: State> RenderCallback<S> for FnMut(
    (
        &mut S,
        &mut Context<backend::Backend, backend::Device, backend::Instance>,
    ),
) -> Result<(), Error> {}

pub trait SetupCallback<S: State, R: Future<Item = S, Error = Error> + Send + 'static, I: IntoFuture<Future = R, Item = S, Error = Error> + 'static>: Send + 'static + FnOnce(
    (
    Arc<SetupContext<backend::Backend, backend::Device, backend::Instance>>
    ),
) -> I {}