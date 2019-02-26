use failure::Context;
use failure::Fail;
use std::fmt::Display;
use std::fmt;
use failure::Backtrace;

#[derive(Debug)]
pub struct CreateEncoderError {
    inner: Context<CreateEncoderErrorKind>
}

impl CreateEncoderError {
    pub fn kind(&self) -> CreateEncoderErrorKind {
        *self.inner.get_context()
    }
}

impl From<CreateEncoderErrorKind> for CreateEncoderError {
    fn from(kind: CreateEncoderErrorKind) -> CreateEncoderError {
        CreateEncoderError { inner: Context::new(kind) }
    }
}

impl From<Context<CreateEncoderErrorKind>> for CreateEncoderError {
    fn from(inner: Context<CreateEncoderErrorKind>) -> CreateEncoderError {
        CreateEncoderError { inner }
    }
}

impl Fail for CreateEncoderError {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for CreateEncoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum CreateEncoderErrorKind {
    #[fail(display = "The swapchain needs to be recreated")]
    RecreateSwapchain,

    #[fail(display = "This error can safely be ignored")]
    Timeout,

    #[fail(display = "Device has been lost")]
    DeviceLost,

    #[fail(display = "Device has been lost")]
    OutOfMemory,
}