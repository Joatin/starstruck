use gfx_hal::pso::AttributeDesc;
use std::fmt::Debug;

/// This trait is used to represent a Vertex. A vertex is a 3 dimensional point in space and is
/// used bu the GPU to create primitives (most often triangles) that is then used to calculate our
/// pixels. A vertex can also have some metadata attached to it like colors or texture coordinates.
pub trait Vertex: Debug + Copy + Send + Sync {
    /// Returns the stride size for this kind of vertex. This is the exact size in bytes for a
    /// single vertex.
    fn stride() -> usize;

    /// Attributes contains some additional info sent to the GPU.
    fn attributes() -> Vec<AttributeDesc>;
}
