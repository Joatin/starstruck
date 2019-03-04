use failure::Error;
use crate::menu::Component;
use crate::context::Context;

pub trait View {
    fn draw(&self, context: &Context) -> Result<(), Error>;
    fn covers_screen(&self) -> bool;
}