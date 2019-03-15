use crate::primitive::Vertex;
use gfx_hal::format::Format;
use gfx_hal::pso::AttributeDesc;
use gfx_hal::pso::Element;
use std::mem::size_of;

#[derive(Debug, Clone, Copy, Default)]
pub struct VertexXYZRGBAUV {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub u: f32,
    pub v: f32,
}

impl VertexXYZRGBAUV {}

pub type Vertex3dColorUv = VertexXYZRGBAUV;

impl Vertex for VertexXYZRGBAUV {
    fn stride() -> usize {
        size_of::<VertexXYZRGBAUV>()
    }

    fn attributes() -> Vec<AttributeDesc> {
        let position_attribute = AttributeDesc {
            location: 0,
            binding: 0,
            element: Element {
                format: Format::Rgb32Float,
                offset: 0,
            },
        };
        let color_attribute = AttributeDesc {
            location: 1,
            binding: 0,
            element: Element {
                format: Format::Rgba32Float,
                offset: (size_of::<f32>() * 3) as _,
            },
        };
        let uv_attribute = AttributeDesc {
            location: 2,
            binding: 0,
            element: Element {
                format: Format::Rg32Float,
                offset: (size_of::<f32>() * 7) as _,
            },
        };
        vec![position_attribute, color_attribute, uv_attribute]
    }
}
