use std::fmt::Debug;
use gfx_hal::pso::AttributeDesc;

pub trait Vertex: Sized + Debug + Copy + Send + Sync {
    fn get_stride() -> usize;
    fn attributes() -> Vec<AttributeDesc>;
}