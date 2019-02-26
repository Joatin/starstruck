use crate::primitive::Vertex;
use std::mem::size_of;
use gfx_hal::pso::AttributeDesc;
use gfx_hal::pso::Element;
use gfx_hal::format::Format;


#[derive(Debug, Default, Clone, Copy)]
pub struct VertexXY {
    pub x: f32,
    pub y: f32
}

impl VertexXY {

}

impl Vertex for VertexXY {
    fn get_stride() -> usize {
        size_of::<f32>() * 2
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