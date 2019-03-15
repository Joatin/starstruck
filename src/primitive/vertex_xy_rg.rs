use crate::graphics::Pipeline;
use crate::graphics::ShaderDescription;
use crate::graphics::ShaderSet;
use crate::primitive::Vertex;
use crate::setup_context::CreateTexturedPipeline;
use crate::setup_context::SetupContext;
use failure::Error;
use futures::Future;
use gfx_hal::format::Format;
use gfx_hal::pso::AttributeDesc;
use gfx_hal::pso::DescriptorType;
use gfx_hal::pso::Element;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;
use std::mem::size_of;
use crate::allocator::GpuAllocator;

#[derive(Debug, Clone, Copy, Default)]
pub struct VertexXYRG {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub g: f32,
}

impl VertexXYRG {}

pub type Vertex2DUV = VertexXYRG;

impl Vertex for VertexXYRG {
    fn stride() -> usize {
        size_of::<Vertex2DUV>()
    }

    fn attributes() -> Vec<AttributeDesc> {
        let position_attribute = AttributeDesc {
            location: 0,
            binding: 0,
            element: Element {
                format: Format::Rg32Float,
                offset: 0,
            },
        };
        let uv_attribute = AttributeDesc {
            location: 1,
            binding: 0,
            element: Element {
                format: Format::Rg32Float,
                offset: (size_of::<f32>() * 2) as _,
            },
        };
        vec![position_attribute, uv_attribute]
    }
}

impl<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>>
CreateTexturedPipeline<VertexXYRG, A, B, D, I> for SetupContext<A, B, D, I>
{
    #[allow(clippy::type_complexity)]
    fn create_textured_pipeline(
        &self,
    ) -> Box<Future<Item = Pipeline<VertexXYRG, A, B, D, I>, Error = Error> + Send> {
        let set = ShaderSet {
            vertex: ShaderDescription {
                spirv: include_bytes!(concat!(env!("OUT_DIR"), "/vertex_xy_rg_textured.vert.spv")),
                push_constant_floats: 16,
                bindings: vec![],
            },
            hull: None,
            domain: None,
            geometry: None,
            fragment: Some(ShaderDescription {
                spirv: include_bytes!(concat!(env!("OUT_DIR"), "/vertex_xy_rg_textured.frag.spv")),
                push_constant_floats: 0,
                bindings: vec![
                    (0, DescriptorType::SampledImage, 1),
                    (1, DescriptorType::Sampler, 1),
                ],
            }),
        };

        Box::new(self.create_pipeline(set))
    }
}
