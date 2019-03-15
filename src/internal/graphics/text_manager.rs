use glyph_brush::GlyphBrushBuilder;
use glyph_brush::Section;
use glyph_brush::Layout;
use glyph_brush::GlyphBrush;
use glyph_brush::rusttype::Scale;
use glyph_brush::BrushError;
use glyph_brush::BrushAction;
use glyph_brush::BuiltInLineBreaker;
use glyph_brush::rusttype::Rect;
use glyph_brush::rusttype::point;
use failure::Error;
use crate::setup_context::SetupContext;
use std::sync::Arc;
use gfx_hal::Instance;
use gfx_hal::Device;
use gfx_hal::Backend;
use crate::allocator::GpuAllocator;
use crate::context::Context;
use crate::graphics::Pipeline;
use crate::primitive::Vertex3dColorUv;
use crate::graphics::ShaderSet;
use crate::graphics::ShaderDescription;
use futures::Future;
use gfx_hal::pso::DescriptorType;
use crate::graphics::Bundle;
use std::sync::Mutex;
use crate::graphics::Texture;
use crate::internal::graphics::Single;
use crate::graphics::R8Unorm;

#[allow(clippy::type_complexity)]
pub struct TextManager<'a, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> {
    glyph_brush: GlyphBrush<'a, ([u16; 6], [Vertex3dColorUv; 4])>,
    dpi: f32,
    pipeline: Pipeline<Vertex3dColorUv, A, B, D, I>,
    bundle: Arc<Mutex<Option<Bundle<u16, Vertex3dColorUv, A, B, D, I>>>>,
    debug_bundle: Bundle<u16, Vertex3dColorUv, A, B, D, I>,
    texture: Arc<Mutex<Texture<R8Unorm, Single, A, B, D, I>>>,
    size: Arc<Mutex<(u32, u32)>>,
    show_debug_view: bool
}

impl<'a, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> TextManager<'a, A, B, D, I> {
    const INDEXES: [u16; 6] = [0, 1, 2, 3, 0, 1];
    const VERTEXES: [Vertex3dColorUv; 4] = [
        Vertex3dColorUv { x: -0.5, y: 0.5, z: 0.0, r: 1.0, g: 1.0, b: 0.0, a: 1.0, u: 0.0, v: 1.0 },
        Vertex3dColorUv { x: 0.5, y: -0.5, z: 0.0, r: 1.0, g: 1.0, b: 0.0, a: 1.0, u: 1.0, v: 0.0 },
        Vertex3dColorUv { x: 0.5, y: 0.5, z: 0.0, r: 1.0, g: 1.0, b: 0.0, a: 1.0, u: 1.0, v: 1.0 },
        Vertex3dColorUv { x: -0.5, y: -0.5, z: 0.0, r: 1.0, g: 1.0, b: 0.0, a: 1.0, u: 0.0, v: 0.0 }
    ];

    pub fn new(setup: Arc<SetupContext<A, B, D, I>>) -> Result<Self, Error> {

        let hack_font: &[u8] = include_bytes!("hack.ttf");
        let glyph_brush = GlyphBrushBuilder::using_font_bytes(hack_font).build();


        let pipeline = setup.create_pipeline(ShaderSet {
            vertex: ShaderDescription {
                spirv: include_bytes!(concat!(env!("OUT_DIR"), "/text.vert.spv")),
                push_constant_floats: 16,
                bindings: vec![]
            },
            hull: None,
            domain: None,
            geometry: None,
            fragment: Some(ShaderDescription {
                spirv: include_bytes!(concat!(env!("OUT_DIR"), "/text.frag.spv")),
                push_constant_floats: 0,
                bindings: vec![
                    (0, DescriptorType::SampledImage, 1),
                    (1, DescriptorType::Sampler, 1),
                ]
            })
        }).wait()?;

        let (width, height) = glyph_brush.texture_dimensions();
        let texture = setup.create_texture_sized::<R8Unorm>(width, height).wait()?;

        pipeline.bind_texture(&texture);

        let debug_bundle = setup.create_bundle(&Self::INDEXES, &Self::VERTEXES).wait()?;

        Ok(Self {
            glyph_brush,
            dpi: setup.dpi() as f32,
            pipeline,
            bundle: Arc::new(Mutex::new(None)),
            texture: Arc::new(Mutex::new(texture)),
            size: Arc::new(Mutex::new((width, height))),
            debug_bundle,
            show_debug_view: false
        })
    }

    fn convert_vertex(glyph_brush::GlyphVertex {
                          mut tex_coords,
                          pixel_coords,
                          bounds,
                          screen_dimensions: (screen_w, screen_h),
                          color,
                          z,
                      }: glyph_brush::GlyphVertex) -> ([u16; 6], [Vertex3dColorUv; 4]) {
            let gl_bounds = Rect {
                min: point(
                    2.0 * (bounds.min.x / screen_w - 0.5),
                    2.0 * (0.5 - bounds.min.y / screen_h),
                ),
                max: point(
                    2.0 * (bounds.max.x / screen_w - 0.5),
                    2.0 * (0.5 - bounds.max.y / screen_h),
                ),
            };

            let mut gl_rect = Rect {
                min: point(
                    2.0 * (pixel_coords.min.x as f32 / screen_w - 0.5),
                    2.0 * (0.5 - pixel_coords.min.y as f32 / screen_h),
                ),
                max: point(
                    2.0 * (pixel_coords.max.x as f32 / screen_w - 0.5),
                    2.0 * (0.5 - pixel_coords.max.y as f32 / screen_h),
                ),
            };

            // handle overlapping bounds, modify uv_rect to preserve texture aspect
            if gl_rect.max.x > gl_bounds.max.x {
                let old_width = gl_rect.width();
                gl_rect.max.x = gl_bounds.max.x;
                tex_coords.max.x = tex_coords.min.x + tex_coords.width() * gl_rect.width() / old_width;
            }
            if gl_rect.min.x < gl_bounds.min.x {
                let old_width = gl_rect.width();
                gl_rect.min.x = gl_bounds.min.x;
                tex_coords.min.x = tex_coords.max.x - tex_coords.width() * gl_rect.width() / old_width;
            }
            // note: y access is flipped gl compared with screen,
            // texture is not flipped (ie is a headache)
            if gl_rect.max.y < gl_bounds.max.y {
                let old_height = gl_rect.height();
                gl_rect.max.y = gl_bounds.max.y;
                tex_coords.max.y = tex_coords.min.y + tex_coords.height() * gl_rect.height() / old_height;
            }
            if gl_rect.min.y > gl_bounds.min.y {
                let old_height = gl_rect.height();
                gl_rect.min.y = gl_bounds.min.y;
                tex_coords.min.y = tex_coords.max.y - tex_coords.height() * gl_rect.height() / old_height;
            }

        gl_rect.max.y *= -1.0;
        gl_rect.min.y *= -1.0;

            let vertexes = [
                Vertex3dColorUv {
                    x: gl_rect.min.x,
                    y: gl_rect.max.y,
                    z,
                    r: color[0],
                    g: color[1],
                    b: color[2],
                    a: color[3],
                    u: tex_coords.min.x,
                    v: tex_coords.max.y,
                },
                Vertex3dColorUv {
                    x: gl_rect.max.x,
                    y: gl_rect.min.y,
                    z,
                    r: color[0],
                    g: color[1],
                    b: color[2],
                    a: color[3],
                    u: tex_coords.max.x,
                    v: tex_coords.min.y,
                },
                Vertex3dColorUv {
                    x: gl_rect.max.x,
                    y: gl_rect.max.y,
                    z,
                    r: color[0],
                    g: color[1],
                    b: color[2],
                    a: color[3],
                    u: tex_coords.max.x,
                    v: tex_coords.max.y,
                },
                Vertex3dColorUv {
                    x: gl_rect.min.x,
                    y: gl_rect.min.y,
                    z,
                    r: color[0],
                    g: color[1],
                    b: color[2],
                    a: color[3],
                    u: tex_coords.min.x,
                    v: tex_coords.min.y,
                }
            ];

            let indexes = [0, 1, 2, 3, 0, 1];

            (indexes, vertexes)
    }

    pub(crate) fn draw(&mut self, context: &mut Context<A, B, D, I>) -> Result<(), Error> {
        {
            let lock  = self.texture.lock().unwrap();
            self.pipeline.bind_texture(&*lock);
        }
        {
            let (new_width, new_height) =  {
                let lock = self.size.lock().unwrap();
                *lock
            };

            let (c_width, c_height) = self.glyph_brush.texture_dimensions();

            if c_width != new_width || c_height != new_height {
                self.glyph_brush.resize_texture(new_width, new_height);
            }
        }

        let mut brush_action;
        let area = context.render_area();
        let width = f64::from(area.width);
        let height = f64::from(area.height);

        loop {
            brush_action = {
                let glyph_brush = &mut self.glyph_brush;
                let texture = &self.texture;
                glyph_brush.process_queued(
                    (width as _, height as _),
                    |rect, tex_data| {
                        trace!("draw glyph");
                        {
                            let lock = texture.lock().unwrap();
                            lock.write_subset((rect.min.x, rect.min.y, rect.width(), rect.height()), tex_data).wait().unwrap();
                        }
                    },
                    Self::convert_vertex,
                )
            };

            match brush_action {
                Ok(_) => break,
                Err(BrushError::TextureTooSmall { suggested, .. }) => {
                    let (new_width, new_height) = suggested;
                    let texture_lock = Arc::clone(&self.texture);
                    let size_lock = Arc::clone(&self.size);

                    context.setup(move |setup| {
                        info!("Resizing glyph texture -> {}x{}", new_width, new_height);
                        setup.create_texture_sized::<R8Unorm>(new_width, new_height).map(move |texture| {
                            {
                                let mut lock = texture_lock.lock().unwrap();
                                *lock = texture;
                            }
                            {
                                let mut lock = size_lock.lock().unwrap();
                                *lock = (new_width, new_height)
                            }
                        })
                    });

                },
            }
        }

        match brush_action? {
            BrushAction::Draw(index_verts) => {
                {
                    let bundle_lock = Arc::clone(&self.bundle);
                    context.setup(move |setup| {
                        let mut bundle = bundle_lock.lock().unwrap();

                        if !index_verts.is_empty() {
                            let mut indexes = Vec::new();
                            let mut vertexes = Vec::new();

                            for (index, vertex) in index_verts {
                                indexes.push(index[0] + vertexes.len() as u16);
                                indexes.push(index[1] + vertexes.len() as u16);
                                indexes.push(index[2] + vertexes.len() as u16);
                                indexes.push(index[3] + vertexes.len() as u16);
                                indexes.push(index[4] + vertexes.len() as u16);
                                indexes.push(index[5] + vertexes.len() as u16);

                                for v in &vertex {
                                    vertexes.push(*v);
                                }
                            }

                            let result = setup.create_bundle_owned(Arc::new(indexes), Arc::new(vertexes)).wait()?;

                            *bundle = Some(result);
                            Ok(())
                        } else {
                            *bundle = None;
                            Ok(())
                        }
                    });
                }


                let bundle = self.bundle.lock().unwrap();
                if let Some(b) = bundle.as_ref() {
                    context.draw(&self.pipeline, b)
                }
            }
            BrushAction::ReDraw => {
                let bundle = self.bundle.lock().unwrap();
                if let Some(b) = bundle.as_ref() {
                    context.draw(&self.pipeline, b)
                }
            }
        };
        if self.show_debug_view {
            context.draw(&self.pipeline, &self.debug_bundle);
        }

        Ok(())
    }

    pub fn draw_text(&mut self, text: &str, size: f32, screen_position: (f32, f32), layout: Layout<BuiltInLineBreaker>) {
        debug_assert!(size > 0.0, "Font size can't be less than zero");

        let dpi = self.dpi;
        self.glyph_brush.queue(Section {
            text,
            screen_position,
            scale: Scale::uniform((size * dpi).round()),
            color: [1.0, 1.0, 1.0, 1.0],
            z: 0.0,
            layout,
            ..Section::default()
        });
    }

    pub fn resize(&mut self) {
        info!("Resize glyphs");
        let (c_width, c_height) = self.glyph_brush.texture_dimensions();
        self.glyph_brush.resize_texture(c_width, c_height);
    }

}