use crate::context::Context;
use crate::graphics::Bundle;
use crate::menu::View;
use failure::Error;

pub struct InitView {}

impl InitView {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {})
    }
}

impl View for InitView {
    fn draw(&self, context: &Context) -> Result<(), Error> {
        Ok(())
    }

    fn covers_screen(&self) -> bool {
        true
    }
}
