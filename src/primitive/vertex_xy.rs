use crate::graphics::Pipeline;
use crate::graphics::ShaderDescription;
use crate::graphics::ShaderSet;
use crate::primitive::Vertex;
use crate::setup_context::CreateDefaultPipeline;
use crate::setup_context::SetupContext;
use failure::Error;
use futures::Future;
use gfx_hal::format::Format;
use gfx_hal::pso::AttributeDesc;
use gfx_hal::pso::Element;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;
use std::mem::size_of;
use std::sync::Arc;

/// A vertex with two floats. This is often used to represent a 2D position
///
/// # Examples
/// ```
/// use starstruck::primitive::VertexXY;
///
/// let vertex = VertexXY {
///     x: 0.0,
///     y: 0.0
/// };
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct VertexXY {
    pub x: f32,
    pub y: f32,
}

impl VertexXY {}

impl Vertex for VertexXY {
    fn stride() -> usize {
        size_of::<VertexXY>()
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
        vec![position_attribute]
    }
}

pub type Vertex2D = VertexXY;

impl<B: Backend, D: Device<B>, I: Instance<Backend = B>> CreateDefaultPipeline<VertexXY, B, D, I>
    for SetupContext<B, D, I>
{
    #[allow(clippy::type_complexity)]
    fn create_default_pipeline(
        &self,
    ) -> Box<Future<Item = Arc<Pipeline<VertexXY, B, D, I>>, Error = Error> + Send> {
        let set = ShaderSet {
            vertex: ShaderDescription {
                spirv: include_bytes!(concat!(env!("OUT_DIR"), "/vertex_xy_default.vert.spv")),
                push_constant_floats: 16,
                bindings: vec![],
            },
            hull: None,
            domain: None,
            geometry: None,
            fragment: Some(ShaderDescription {
                spirv: include_bytes!(concat!(env!("OUT_DIR"), "/vertex_xy_default.frag.spv")),
                push_constant_floats: 0,
                bindings: vec![],
            }),
        };

        Box::new(self.create_pipeline(set))
    }
}

#[cfg(test)]
mod tests {
    use crate::primitive::Vertex;
    use crate::primitive::VertexXY;
    use gfx_hal::format::Format;
    use gfx_hal::pso::AttributeDesc;
    use gfx_hal::pso::Element;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_should_return_correct_stride() {
        assert_eq!(8, VertexXY::stride())
    }

    #[test]
    fn it_should_return_correct_attributes() {
        assert_eq!(
            vec![AttributeDesc {
                location: 0,
                binding: 0,
                element: Element {
                    format: Format::Rg32Float,
                    offset: 0,
                },
            }],
            VertexXY::attributes()
        )
    }

}
