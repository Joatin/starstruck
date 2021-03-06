use crate::graphics::Bundle;
use crate::graphics::Pipeline;
use crate::graphics::ShaderDescription;
use crate::graphics::ShaderSet;
use crate::primitive::Vertex;
use crate::setup_context::CreateBundleFromObj;
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
use obj::Obj;
use obj::SimplePolygon;
use std::io::BufReader;
use std::mem::size_of;
use std::mem::transmute;
use std::sync::Arc;
use crate::allocator::GpuAllocator;

#[derive(Debug, Default, Clone, Copy)]
pub struct VertexXYZ {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl VertexXYZ {}

impl Vertex for VertexXYZ {
    fn stride() -> usize {
        size_of::<f32>() * 3
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
        vec![position_attribute]
    }
}

pub type Vertex3D = VertexXYZ;

impl<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> CreateDefaultPipeline<VertexXYZ, A, B, D, I>
    for SetupContext<A, B, D, I>
{
    #[allow(clippy::type_complexity)]
    fn create_default_pipeline(
        &self,
    ) -> Box<Future<Item = Pipeline<VertexXYZ, A, B, D, I>, Error = Error> + Send> {
        let set = ShaderSet {
            vertex: ShaderDescription {
                spirv: include_bytes!(concat!(env!("OUT_DIR"), "/vertex_xyz_default.vert.spv")),
                push_constant_floats: 16,
                bindings: vec![],
            },
            hull: None,
            domain: None,
            geometry: None,
            fragment: Some(ShaderDescription {
                spirv: include_bytes!(concat!(env!("OUT_DIR"), "/vertex_xyz_default.frag.spv")),
                push_constant_floats: 0,
                bindings: vec![],
            }),
        };

        Box::new(self.create_pipeline(set))
    }
}

impl<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>>
    CreateBundleFromObj<u16, VertexXYZ, A, B, D, I> for SetupContext<A, B, D, I>
{

    #[allow(clippy::type_complexity)]
    fn create_bundle_from_obj(
        &self,
        data: &[u8],
    ) -> Box<Future<Item = Bundle<u16, VertexXYZ, A, B, D, I>, Error = Error> + Send> {
        let mut reader = BufReader::new(data);
        match Obj::load_buf(&mut reader) {
            Ok(obj_data) => {
                let vertices: Vec<VertexXYZ> = unsafe { transmute(obj_data.position) };
                let indexes: Vec<u16> = obj_data.objects[0].groups[0]
                    .polys
                    .clone()
                    .into_iter()
                    .flat_map(|poly: SimplePolygon| {
                        let array: Vec<u16> = poly.into_iter().map(|p| p.0 as _).collect();
                        array
                    })
                    .collect();

                Box::new(self.create_bundle_owned(Arc::new(indexes), Arc::new(vertices)))
            }
            Err(err) => Box::new(futures::done(Err(err.into()))),
        }
    }
}

impl From<[f32; 3]> for VertexXYZ {
    fn from(data: [f32; 3]) -> Self {
        Self {
            x: data[0],
            y: data[1],
            z: data[2],
        }
    }
}
