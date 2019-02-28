use crate::primitive::Vertex;
use std::mem::size_of;
use gfx_hal::pso::AttributeDesc;
use gfx_hal::pso::Element;
use gfx_hal::format::Format;
use crate::setup_context::CreateDefaultPipeline;
use crate::setup_context::SetupContext;
use futures::Future;
use std::sync::Arc;
use crate::graphics::Pipeline;
use failure::Error;
use crate::graphics::ShaderSet;
use crate::graphics::ShaderDescription;

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
    pub y: f32
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

impl CreateDefaultPipeline<VertexXY> for SetupContext {
    fn create_default_pipeline(&self) -> Box<Future<Item=Arc<Pipeline<VertexXY>>, Error=Error> + Send> {
        let set = ShaderSet {
            vertex: ShaderDescription { spirv: include_bytes!(concat!(env!("OUT_DIR"), "/vertex_xy_default.vert.spv")), constant_byte_size: 16 },
            hull: None,
            domain: None,
            geometry: None,
            fragment: Some(ShaderDescription { spirv: include_bytes!(concat!(env!("OUT_DIR"), "/vertex_xy_default.frag.spv")), constant_byte_size: 0 })
        };

        Box::new(self.create_pipeline(set))
    }
}

#[cfg(test)]
mod tests {
    use crate::primitive::VertexXY;
    use crate::primitive::Vertex;

    #[test]
    fn it_should_return_correct_stride() {
        assert_eq!(8, VertexXY::stride())
    }

}