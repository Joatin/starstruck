use crate::graphics::ShaderDescription;
use crate::graphics::ShaderSet;
use crate::primitive::Vertex;
use colored::*;
use failure::Error;
use gfx_hal::device::Device;
use gfx_hal::pass::Subpass;
use gfx_hal::pso::BakedStates;
use gfx_hal::pso::BasePipeline;
use gfx_hal::pso::BlendDesc;
use gfx_hal::pso::BlendOp;
use gfx_hal::pso::BlendState;
use gfx_hal::pso::ColorBlendDesc;
use gfx_hal::pso::ColorMask;
use gfx_hal::pso::Comparison;
use gfx_hal::pso::DepthStencilDesc;
use gfx_hal::pso::DepthTest;
use gfx_hal::pso::DescriptorSetLayoutBinding;
use gfx_hal::pso::EntryPoint;
use gfx_hal::pso::Factor;
use gfx_hal::pso::GraphicsPipelineDesc;
use gfx_hal::pso::GraphicsShaderSet;
use gfx_hal::pso::InputAssemblerDesc;
use gfx_hal::pso::LogicOp;
use gfx_hal::pso::Multisampling;
use gfx_hal::pso::PipelineCreationFlags;
use gfx_hal::pso::Rasterizer;
use gfx_hal::pso::Rect;
use gfx_hal::pso::ShaderStageFlags;
use gfx_hal::pso::Specialization;
use gfx_hal::pso::StencilTest;
use gfx_hal::pso::VertexBufferDesc;
use gfx_hal::pso::Viewport;
use gfx_hal::window::Extent2D;
use gfx_hal::Backend;
use gfx_hal::Primitive;
use std::mem::ManuallyDrop;
use std::sync::Arc;

pub struct PipelineBundle {
    descriptor_layouts: Vec<<backend::Backend as Backend>::DescriptorSetLayout>,
    layout: ManuallyDrop<<backend::Backend as Backend>::PipelineLayout>,
    pipeline: ManuallyDrop<<backend::Backend as Backend>::GraphicsPipeline>,
    device: Arc<backend::Device>,
}

impl PipelineBundle {
    pub fn new<T: Vertex>(
        device: Arc<backend::Device>,
        render_pass: &<backend::Backend as Backend>::RenderPass,
        render_area: Extent2D,
        set: &ShaderSet,
    ) -> Result<Self, Error> {
        info!("{}", "Creating new pipeline".green());

        let (descriptor_layouts, layout, pipeline) =
            Self::create::<T>(&device, render_pass, render_area, &set)?;
        Ok(Self {
            descriptor_layouts,
            layout: ManuallyDrop::new(layout),
            pipeline: ManuallyDrop::new(pipeline),
            device: Arc::clone(&device),
        })
    }

    pub fn layout(&self) -> &<backend::Backend as Backend>::PipelineLayout {
        &self.layout
    }

    #[allow(clippy::type_complexity)]
    fn create<T: Vertex>(
        device: &backend::Device,
        render_pass: &<backend::Backend as Backend>::RenderPass,
        render_area: Extent2D,
        set: &ShaderSet,
    ) -> Result<
        (
            Vec<<backend::Backend as Backend>::DescriptorSetLayout>,
            <backend::Backend as Backend>::PipelineLayout,
            <backend::Backend as Backend>::GraphicsPipeline,
        ),
        Error,
    > {
        let shader_modules = Self::create_shader_modules(&device, set)?;
        let result = {
            let shaders = Self::create_graphics_shader_set(&shader_modules)?;
            let rasterizer = Self::create_rasterizer();
            let descriptor_layouts = Self::create_descriptor_layouts(&device)?;
            let layout = Self::create_pipeline_layout(&device, &descriptor_layouts)?;

            let pipeline = Self::create_pipeline::<T>(
                &device,
                &render_pass,
                &layout,
                shaders,
                rasterizer,
                render_area,
            )?;

            (descriptor_layouts, layout, pipeline)
        };

        Self::destroy_shader_modules(&device, shader_modules);

        Ok(result)
    }

    pub fn pipeline(&self) -> &<backend::Backend as Backend>::GraphicsPipeline {
        &self.pipeline
    }

    fn create_pipeline<T: Vertex>(
        device: &backend::Device,
        render_pass: &<backend::Backend as Backend>::RenderPass,
        layout: &<backend::Backend as Backend>::PipelineLayout,
        shaders: GraphicsShaderSet<backend::Backend>,
        rasterizer: Rasterizer,
        render_area: Extent2D,
    ) -> Result<<backend::Backend as Backend>::GraphicsPipeline, Error> {
        let input_assembler = InputAssemblerDesc::new(Primitive::TriangleList);

        let vertex_buffers: Vec<VertexBufferDesc> = vec![VertexBufferDesc {
            binding: 0,
            stride: T::stride() as u32,
            rate: 0,
        }];

        let depth_stencil = DepthStencilDesc {
            depth: DepthTest::On {
                fun: Comparison::LessEqual,
                write: true,
            },
            depth_bounds: false,
            stencil: StencilTest::Off,
        };

        let blender = {
            let blend_state = BlendState::On {
                color: BlendOp::Add {
                    src: Factor::One,
                    dst: Factor::Zero,
                },
                alpha: BlendOp::Add {
                    src: Factor::One,
                    dst: Factor::Zero,
                },
            };
            BlendDesc {
                logic_op: Some(LogicOp::Copy),
                targets: vec![ColorBlendDesc(ColorMask::ALL, blend_state)],
            }
        };

        let render_area_rect = Rect {
            x: 0,
            y: 0,
            w: render_area.width as _,
            h: render_area.height as _,
        };

        let baked_states = BakedStates {
            viewport: Some(Viewport {
                rect: render_area_rect,
                depth: (0.0..1.0),
            }),
            scissor: Some(render_area_rect),
            blend_color: None,
            depth_bounds: None,
        };

        let desc = GraphicsPipelineDesc {
            shaders,
            rasterizer,
            vertex_buffers,
            attributes: T::attributes(),
            input_assembler,
            blender,
            depth_stencil,
            multisampling: Some(Multisampling {
                rasterization_samples: 4,
                sample_shading: None,
                sample_mask: 0,
                alpha_coverage: false,
                alpha_to_one: false,
            }),
            baked_states,
            layout: &layout,
            subpass: Subpass {
                index: 0,
                main_pass: render_pass,
            },
            flags: PipelineCreationFlags::empty(),
            parent: BasePipeline::None,
        };

        Ok(unsafe { device.create_graphics_pipeline(&desc, None)? })
    }

    fn create_rasterizer() -> Rasterizer {
        Rasterizer::FILL
    }

    fn create_pipeline_layout(
        device: &backend::Device,
        descriptor_set_layouts: &[<backend::Backend as Backend>::DescriptorSetLayout],
    ) -> Result<<backend::Backend as Backend>::PipelineLayout, Error> {
        let push_constants = vec![
            (ShaderStageFlags::VERTEX, 0..16),
            (ShaderStageFlags::FRAGMENT, 0..1),
        ];
        Ok(unsafe { device.create_pipeline_layout(descriptor_set_layouts, push_constants)? })
    }

    fn create_descriptor_layouts(
        device: &backend::Device,
    ) -> Result<Vec<<backend::Backend as Backend>::DescriptorSetLayout>, Error> {
        let bindings = Vec::<DescriptorSetLayoutBinding>::new();
        let immutable_samplers = Vec::<<backend::Backend as Backend>::Sampler>::new();
        Ok(vec![unsafe {
            device.create_descriptor_set_layout(bindings, immutable_samplers)?
        }])
    }

    fn create_shader_modules(
        device: &backend::Device,
        set: &ShaderSet,
    ) -> Result<[Option<<backend::Backend as Backend>::ShaderModule>; 5], Error> {
        Ok([
            Some(unsafe { device.create_shader_module(set.vertex.spirv)? }),
            Self::map_to_shader_module(device, &set.hull)?,
            Self::map_to_shader_module(device, &set.domain)?,
            Self::map_to_shader_module(device, &set.geometry)?,
            Self::map_to_shader_module(device, &set.fragment)?,
        ])
    }

    fn destroy_shader_modules(
        device: &backend::Device,
        mut shader_modules: [Option<<backend::Backend as Backend>::ShaderModule>; 5],
    ) {
        for shader in &mut shader_modules {
            if let Some(s) = shader.take() {
                unsafe { device.destroy_shader_module(s) }
            }
        }
    }

    fn create_graphics_shader_set(
        modules: &[Option<<backend::Backend as Backend>::ShaderModule>],
    ) -> Result<GraphicsShaderSet<backend::Backend>, Error> {
        let shaders = GraphicsShaderSet {
            vertex: Self::create_shader_entry_point(&modules[0])
                .expect("Vertex is always defined, this can't panic"),
            hull: Self::create_shader_entry_point(&modules[1]),
            domain: Self::create_shader_entry_point(&modules[2]),
            geometry: Self::create_shader_entry_point(&modules[3]),
            fragment: Self::create_shader_entry_point(&modules[4]),
        };

        Ok(shaders)
    }

    fn map_to_shader_module(
        device: &backend::Device,
        desc: &Option<ShaderDescription>,
    ) -> Result<Option<<backend::Backend as Backend>::ShaderModule>, Error> {
        match desc {
            Some(d) => Ok(Some(unsafe { device.create_shader_module(d.spirv)? })),
            None => Ok(None),
        }
    }

    fn create_shader_entry_point(
        shader_module: &Option<<backend::Backend as Backend>::ShaderModule>,
    ) -> Option<EntryPoint<backend::Backend>> {
        match shader_module {
            Some(ref m) => Some(EntryPoint {
                entry: "main",
                module: m,
                specialization: Specialization {
                    constants: &[],
                    data: &[],
                },
            }),
            None => None,
        }
    }
}

impl Drop for PipelineBundle {
    fn drop(&mut self) {
        use core::ptr::read;

        info!("{}", "Dropping Pipeline".red());

        let device = &self.device;
        let layout = &self.layout;
        let pipeline = &self.pipeline;
        let layouts = &mut self.descriptor_layouts;

        for desc in layouts.drain(..) {
            unsafe { self.device.destroy_descriptor_set_layout(desc) };
        }

        unsafe {
            device.destroy_pipeline_layout(ManuallyDrop::into_inner(read(layout)));
            device.destroy_graphics_pipeline(ManuallyDrop::into_inner(read(pipeline)));
        }
    }
}
