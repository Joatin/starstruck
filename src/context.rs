use crate::input::UserInput;
use gfx_hal::command::RenderPassInlineEncoder;

pub struct Context<'a> {
    input: UserInput,
    encoder: RenderPassInlineEncoder<'a, backend::Backend>
}

impl<'a> Context<'a> {
    pub fn new(input: UserInput, encoder: RenderPassInlineEncoder<'a, backend::Backend>) -> Self {
        Context {
            input,
            encoder
        }
    }
}