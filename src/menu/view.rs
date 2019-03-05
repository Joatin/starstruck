use crate::context::Context;
use failure::Error;

pub trait View {
    fn draw(&self, context: &Context) -> Result<(), Error>;
    fn covers_screen(&self) -> bool;
}
