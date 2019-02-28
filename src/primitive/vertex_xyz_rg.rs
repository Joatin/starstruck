use crate::primitive::Vertex;
use std::mem::size_of;
use gfx_hal::format::Format;
use gfx_hal::pso::AttributeDesc;
use gfx_hal::pso::Element;
use crate::setup_context::CreateDefaultPipeline;
use crate::setup_context::SetupContext;
use futures::Future;
use crate::graphics::ShaderSet;
use crate::graphics::ShaderDescription;
use crate::graphics::Bundle;
use crate::setup_context::CreateBundleFromObj;
use obj::Obj;
use obj::SimplePolygon;
use std::mem::transmute;
use std::io::BufReader;
use std::sync::Arc;
use crate::graphics::Pipeline;
use failure::Error;

#[derive(Debug, Clone, Copy, Default)]
pub struct VertexXYZRG {
    x: f32,
    y: f32,
    z: f32,
    u: f32,
    v: f32
}

impl VertexXYZRG {}

pub type Vertex3DUV = VertexXYZRG;

impl Vertex for VertexXYZRG {
    fn stride() -> usize {
        size_of::<Vertex3DUV>()
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
        let uv_attribute = AttributeDesc {
            location: 1,
            binding: 0,
            element: Element {
                format: Format::Rg32Float,
                offset: (size_of::<f32>() * 3) as _,
            },
        };
        vec![position_attribute, uv_attribute]
    }
}

impl CreateDefaultPipeline<VertexXYZRG> for SetupContext {
    fn create_default_pipeline(
        &self,
    ) -> Box<Future<Item = Arc<Pipeline<VertexXYZRG>>, Error = Error> + Send> {
        let set = ShaderSet {
            vertex: ShaderDescription {
                spirv: include_bytes!(concat!(env!("OUT_DIR"), "/vertex_xyz_rg_default.vert.spv")),
                constant_byte_size: 16,
            },
            hull: None,
            domain: None,
            geometry: None,
            fragment: Some(ShaderDescription {
                spirv: include_bytes!(concat!(env!("OUT_DIR"), "/vertex_xyz_rg_default.frag.spv")),
                constant_byte_size: 0,
            }),
        };

        Box::new(self.create_pipeline(set))
    }
}

impl CreateBundleFromObj<u16, VertexXYZRG> for SetupContext {
    fn create_bundle_from_obj(
        &self,
        data: &[u8],
    ) -> Box<Future<Item = Bundle<u16, VertexXYZRG>, Error = Error> + Send> {
        let mut reader = BufReader::new(data);
        match Obj::load_buf(&mut reader) {
            Ok(obj_data) => {
                let mut vertices: Vec<VertexXYZRG> = Vec::with_capacity(obj_data.position.len() * 2);
                let mut indexes: Vec<u16> = Vec::with_capacity(obj_data.objects[0].groups[0]
                    .polys.len());

                let polys: &[SimplePolygon] = &obj_data.objects[0].groups[0].polys;
                let mut i = 0;
                for poly in polys {
                    indexes.push(i);
                    i += 1;
                    vertices.push(Vertex3DUV {
                        x: obj_data.position[poly[0].0][0],
                        y: obj_data.position[poly[0].0][1],
                        z: obj_data.position[poly[0].0][2],
                        u: obj_data.texture[poly[0].1.unwrap()][0],
                        v: obj_data.texture[poly[0].1.unwrap()][1]
                    });
                    indexes.push(i);
                    i += 1;
                    vertices.push(Vertex3DUV {
                        x: obj_data.position[poly[1].0][0],
                        y: obj_data.position[poly[1].0][1],
                        z: obj_data.position[poly[1].0][2],
                        u: obj_data.texture[poly[1].1.unwrap()][0],
                        v: obj_data.texture[poly[1].1.unwrap()][1]
                    });
                    indexes.push(i);
                    i += 1;
                    vertices.push(Vertex3DUV {
                        x: obj_data.position[poly[2].0][0],
                        y: obj_data.position[poly[2].0][1],
                        z: obj_data.position[poly[2].0][2],
                        u: obj_data.texture[poly[2].1.unwrap()][0],
                        v: obj_data.texture[poly[2].1.unwrap()][1]
                    })
                }

                Box::new(self.create_bundle_owned(Arc::new(indexes), Arc::new(vertices)))
            }
            Err(err) => Box::new(futures::done(Err(err.into()))),
        }
    }
}