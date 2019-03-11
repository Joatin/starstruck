use crate::internal::graphics::BufferBundle;
use crate::internal::graphics::GraphicsState;
use crate::internal::graphics::GPU;
use crate::primitive::Index;
use crate::primitive::Vertex;
use arrayvec::ArrayVec;
use failure::Error;
use futures::Future;
use gfx_hal::buffer::IndexBufferView;
use gfx_hal::buffer::Usage as BufferUsage;
use gfx_hal::command::RenderPassInlineEncoder;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::IndexType;
use gfx_hal::Instance;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::sync::Arc;
use crate::allocator::GpuAllocator;
use crate::allocator::DefaultGpuAllocator;
use crate::allocator::DefaultChunk;

/// A bundle contains both the vertexes and indexes needed to render a entity
pub struct Bundle<
    In: Index,
    V: Vertex,
    A: GpuAllocator<B, D> = DefaultGpuAllocator<DefaultChunk<backend::Backend, backend::Device>, backend::Backend, backend::Device>,
    B: Backend = backend::Backend,
    D: Device<B> = backend::Device,
    I: Instance<Backend = B> = backend::Instance,
> {
    index_buffer_bundle: BufferBundle<A, B, D, I, GPU, In>,
    vertex_buffer_bundle: BufferBundle<A, B, D, I, GPU, V>,
    index_count: u32,
}

impl<In: Index, V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>>
    Bundle<In, V, A, B, D, I>
{
    pub(crate) fn new(
        state: Arc<GraphicsState<A, B, D, I>>,
        indexes: Arc<Vec<In>>,
        vertexes: Arc<Vec<V>>,
    ) -> impl Future<Item = Self, Error = Error> + Send {
        let index_count = indexes.len() as u32;
        let index_buffer_bundle =
            BufferBundle::<A, B, D, I, GPU, In>::new(Arc::clone(&state), BufferUsage::VERTEX, indexes);
        let vertex_buffer_bundle =
            BufferBundle::<A, B, D, I, GPU, V>::new(state, BufferUsage::INDEX, vertexes);

        index_buffer_bundle
            .join(vertex_buffer_bundle)
            .map(move |(index, vert)| Self {
                index_buffer_bundle: index,
                vertex_buffer_bundle: vert,
                index_count,
            })
    }

    pub fn index_count(&self) -> u32 {
        self.index_count
    }
}

fn bind_vertex_bundle<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>, V: Vertex>(
    encoder: &mut RenderPassInlineEncoder<B>,
    bundle: &BufferBundle<A, B, D, I, GPU, V>,
) {
    // Here we must force the Deref impl of ManuallyDrop to play nice.
    let buffer_ref: &B::Buffer = &bundle.buffer;
    let buffers: ArrayVec<[_; 1]> = [(buffer_ref, 0)].into();
    unsafe {
        encoder.bind_vertex_buffers(0, buffers);
    }
}

fn bind_index_bundle<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>, In: Index>(
    encoder: &mut RenderPassInlineEncoder<B>,
    bundle: &BufferBundle<A, B, D, I, GPU, In>,
    index_type: IndexType,
) {
    // Here we must force the Deref impl of ManuallyDrop to play nice.
    let buffer_ref: &B::Buffer = &bundle.buffer;
    unsafe {
        encoder.bind_index_buffer(IndexBufferView {
            buffer: buffer_ref,
            offset: 0,
            index_type,
        });
    }
}

pub trait BundleEncoderExt<In: Index, V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>>
{
    fn bind_bundle(&mut self, bundle: &Bundle<In, V, A, B, D, I>);
}

impl<'a, V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>>
    BundleEncoderExt<u16, V, A, B, D, I> for RenderPassInlineEncoder<'a, B>
{
    fn bind_bundle(&mut self, bundle: &Bundle<u16, V, A, B, D, I>) {
        bind_index_bundle(self, &bundle.index_buffer_bundle, IndexType::U16);
        bind_vertex_bundle(self, &bundle.vertex_buffer_bundle);
    }
}

impl<'a, V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>>
    BundleEncoderExt<u32, V, A, B, D, I> for RenderPassInlineEncoder<'a, B>
{
    fn bind_bundle(&mut self, bundle: &Bundle<u32, V, A, B, D, I>) {
        bind_index_bundle(self, &bundle.index_buffer_bundle, IndexType::U32);
        bind_vertex_bundle(self, &bundle.vertex_buffer_bundle);
    }
}

impl<In: Index, V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Debug
    for Bundle<In, V, A, B, D, I>
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self.index_buffer_bundle)?;
        write!(f, "{:?}", self.vertex_buffer_bundle)?;
        write!(f, "{:?}", self.index_count)?;
        Ok(())
    }
}
