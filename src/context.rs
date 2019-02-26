use crate::input::UserInput;
use gfx_hal::command::RenderPassInlineEncoder;
use crate::setup_context::SetupContext;
use crate::primitive::Vertex;
use crate::graphics::Bundle;
use crate::primitive::Index;
use crate::graphics::BundleEncoderExt;
use crate::graphics::Pipeline;
use crate::graphics::PipelineEncoderExt;

pub struct Context<'a> {
    setup_context: &'a SetupContext,
    input: UserInput,
    encoder: RenderPassInlineEncoder<'a, backend::Backend>
}

impl<'a> Context<'a> {
    pub(crate) fn new(input: UserInput, setup_context: &'a SetupContext, encoder: RenderPassInlineEncoder<'a, backend::Backend>) -> Self {
        Context {
            input,
            encoder,
            setup_context
        }
    }

    pub fn input(&self) -> &UserInput {
        &self.input
    }

    pub fn setup_context(&self) -> &SetupContext {
        self.setup_context
    }

    pub fn draw<I: Index, V: Vertex>(&mut self, pipeline: &Pipeline<V> , bundle: &Bundle<I, V>)
        where RenderPassInlineEncoder<'a, backend::Backend>: BundleEncoderExt<I, V>
    {
        self.encoder.bind_pipeline(pipeline);
        self.encoder.bind_bundle(bundle);
        unsafe { self.encoder.draw_indexed(0..bundle.index_count(), 0, 0..1) }
    }
}