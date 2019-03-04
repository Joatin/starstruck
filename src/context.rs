use crate::graphics::Bundle;
use crate::graphics::BundleEncoderExt;
use crate::graphics::Camera;
use crate::graphics::Pipeline;
use crate::graphics::PipelineEncoderExt;
use crate::input::UserInput;
use crate::internal::Mat4Ext;
use crate::primitive::Index;
use crate::primitive::Vertex;
use crate::setup_context::SetupContext;
use gfx_hal::command::RenderPassInlineEncoder;
use gfx_hal::pso::ShaderStageFlags;
use gfx_hal::window::Extent2D;
use vek::geom::FrustumPlanes;
use vek::Mat4;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;

pub struct Context<'a, B: Backend = backend::Backend, D: Device<B> = backend::Device, I: Instance<Backend=B> = backend::Instance> {
    setup_context: &'a SetupContext<B, D, I>,
    input: UserInput,
    encoder: RenderPassInlineEncoder<'a, B>,
    base_projection: Mat4<f32>,
    render_area: Extent2D,
}

impl<'a, B: Backend, D: Device<B>, I: Instance<Backend=B>> Context<'a, B, D, I> {
    pub(crate) fn new(
        input: UserInput,
        setup_context: &'a SetupContext<B, D, I>,
        encoder: RenderPassInlineEncoder<'a, B>,
        render_area: Extent2D,
    ) -> Self {
        let ratio = (((render_area.width as f32 / render_area.height as f32) - 1.0) / 2.0) + 1.0;
        Context {
            input,
            encoder,
            setup_context,
            render_area,
            base_projection: Mat4::<f32>::orthographic_lh_zo(FrustumPlanes {
                left: ratio * -1.0,
                right: ratio,
                bottom: -1.,
                top: 1.,
                near: 0.,
                far: 100.,
            }),
        }
    }

    pub fn render_area(&self) -> Extent2D {
        self.render_area
    }

    pub fn input(&self) -> &UserInput {
        &self.input
    }

    pub fn setup_context(&self) -> &SetupContext<B, D, I> {
        self.setup_context
    }

    pub fn draw<In: Index, V: Vertex>(&mut self, pipeline: &Pipeline<V, B, D, I>, bundle: &Bundle<In, V, B, D, I>)
    where
        RenderPassInlineEncoder<'a, B>: BundleEncoderExt<In, V, B, D, I>,
    {
        self.encoder.bind_pipeline(pipeline);
        self.encoder.bind_bundle(bundle);

        unsafe {
            let mat_data = self.base_projection.as_push_constant_data();
            self.encoder
                .bind_push_constant(pipeline, ShaderStageFlags::VERTEX, 0, mat_data);
            self.encoder.draw_indexed(0..bundle.index_count(), 0, 0..1)
        }
    }

    pub fn draw_with_camera<In: Index, V: Vertex>(
        &mut self,
        pipeline: &Pipeline<V, B, D, I>,
        bundle: &Bundle<In, V, B, D, I>,
        camera: &Camera
    ) where
        RenderPassInlineEncoder<'a, B>: BundleEncoderExt<In, V, B, D, I>,
    {
        self.encoder.bind_pipeline(pipeline);
        self.encoder.bind_bundle(bundle);

        unsafe {
            let mut mat = camera.projection_view();
            let mat_data = mat.as_push_constant_data();
            self.encoder
                .bind_push_constant(pipeline, ShaderStageFlags::VERTEX, 0, mat_data);
            self.encoder.draw_indexed(0..bundle.index_count(), 0, 0..1)
        }
    }
}
