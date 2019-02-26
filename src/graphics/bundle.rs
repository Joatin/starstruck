use crate::internal::graphics::BufferBundle;
use crate::primitive::Vertex;
use failure::Error;
use gfx_hal::Adapter;
use crate::primitive::Index;
use arrayvec::ArrayVec;
use gfx_hal::Backend;
use gfx_hal::buffer::IndexBufferView;
use gfx_hal::IndexType;
use gfx_hal::command::RenderPassInlineEncoder;
use std::marker::PhantomData;
use futures::Future;
use std::sync::Arc;


pub struct Bundle<I: Index, V: Vertex> {
    index_buffer_bundle: BufferBundle,
    vertex_buffer_bundle: BufferBundle,
    index_count: u32,
    phantom_index: PhantomData<I>,
    phantom_vertex: PhantomData<V>
}

impl<I: Index, V: Vertex> Bundle<I, V> {
    pub(crate) fn new<'a>(adapter: Arc<Adapter<backend::Backend>>, indexes: &'a [I], vertexes: &'a [V]) -> impl Future<Item=Self, Error=Error> + 'a + Send
    {
        let index_count = indexes.len() as u32;
        let index_buffer_bundle = BufferBundle::new_index(Arc::clone(&adapter), indexes);
        let vertex_buffer_bundle = BufferBundle::new_vertex(adapter, vertexes);

        Box::new(index_buffer_bundle.join(vertex_buffer_bundle).map(move |(index, vert)| {
            Self {
                index_buffer_bundle: index,
                vertex_buffer_bundle: vert,
                index_count,
                phantom_index: PhantomData,
                phantom_vertex: PhantomData,
            }
        }))
    }

    pub fn index_count(&self) -> u32 {
        self.index_count
    }
}

fn bind_vertex_bundle(encoder: &mut RenderPassInlineEncoder<backend::Backend>, bundle: &BufferBundle) {
    // Here we must force the Deref impl of ManuallyDrop to play nice.
    let buffer_ref: &<backend::Backend as Backend>::Buffer = &bundle.buffer;
    let buffers: ArrayVec<[_; 1]> = [(buffer_ref, 0)].into();
    unsafe { encoder.bind_vertex_buffers(0, buffers); }
}

fn bind_index_bundle(encoder: &mut RenderPassInlineEncoder<backend::Backend>, bundle: &BufferBundle, index_type: IndexType) {
    // Here we must force the Deref impl of ManuallyDrop to play nice.
    let buffer_ref: &<backend::Backend as Backend>::Buffer = &bundle.buffer;
    unsafe {
        encoder.bind_index_buffer(IndexBufferView {
            buffer: buffer_ref,
            offset: 0,
            index_type
        });
    }
}

pub trait BundleEncoderExt<I: Index, V: Vertex> {
    fn bind_bundle(&mut self, bundle: &Bundle<I, V>);
}

impl<'a, V: Vertex> BundleEncoderExt<u16, V> for RenderPassInlineEncoder<'a, backend::Backend> {
    fn bind_bundle(&mut self, bundle: &Bundle<u16, V>) {
        bind_index_bundle(self, &bundle.index_buffer_bundle, IndexType::U16);
        bind_vertex_bundle(self, &bundle.vertex_buffer_bundle);
    }
}

impl<'a, V: Vertex> BundleEncoderExt<u32, V> for RenderPassInlineEncoder<'a, backend::Backend> {
    fn bind_bundle(&mut self, bundle: &Bundle<u32, V>) {
        bind_index_bundle(self, &bundle.index_buffer_bundle, IndexType::U32);
        bind_vertex_bundle(self, &bundle.vertex_buffer_bundle);
    }
}