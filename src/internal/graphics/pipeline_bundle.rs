use crate::graphics::ShaderDescription;
use crate::graphics::ShaderSet;
use crate::internal::graphics::GraphicsState;
use crate::internal::graphics::PipelineLayoutBundle;
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
use gfx_hal::pso::Descriptor;
use gfx_hal::pso::DescriptorArrayIndex;
use gfx_hal::pso::DescriptorBinding;
use gfx_hal::pso::EntryPoint;
use gfx_hal::pso::GraphicsPipelineDesc;
use gfx_hal::pso::GraphicsShaderSet;
use gfx_hal::pso::InputAssemblerDesc;
use gfx_hal::pso::LogicOp;
use gfx_hal::pso::Multisampling;
use gfx_hal::pso::PipelineCreationFlags;
use gfx_hal::pso::Rasterizer;
use gfx_hal::pso::Specialization;
use gfx_hal::pso::StencilTest;
use gfx_hal::pso::VertexBufferDesc;
use gfx_hal::Backend;
use gfx_hal::Instance;
use gfx_hal::Primitive;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use crate::allocator::GpuAllocator;

pub struct PipelineBundle<V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> {
    pipeline_layout: PipelineLayoutBundle<A, B, D, I>,
    pipeline: ManuallyDrop<B::GraphicsPipeline>,
    state: Arc<GraphicsState<A, B, D, I>>,
    phantom: PhantomData<V>,
}

impl<V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> PipelineBundle<V, A, B, D, I> {
    pub fn new(
        state: Arc<GraphicsState<A, B, D, I>>,
        render_pass: &B::RenderPass,
        set: &ShaderSet,
    ) -> Result<Self, Error> {
        let pipeline_layout = PipelineLayoutBundle::new(Arc::clone(&state), set)?;

        info!("{}", "Creating new pipeline".green());

        let pipeline = Self::create(
            &state.device(),
            render_pass,
            &set,
            pipeline_layout.layout(),
        )?;
        Ok(Self {
            pipeline_layout,
            pipeline: ManuallyDrop::new(pipeline),
            state,
            phantom: PhantomData,
        })
    }

    pub fn layout(&self) -> &B::PipelineLayout {
        &self.pipeline_layout.layout()
    }

    pub fn descriptor_set(&self) -> &B::DescriptorSet {
        &self.pipeline_layout.descriptor_set()
    }

    pub fn bind_assets(
        &self,
        descriptors: Vec<(DescriptorBinding, DescriptorArrayIndex, Descriptor<B>)>,
    ) {
        self.pipeline_layout.bind_assets(descriptors);
    }

    #[allow(clippy::type_complexity)]
    fn create(
        device: &D,
        render_pass: &B::RenderPass,
        set: &ShaderSet,
        layout: &B::PipelineLayout,
    ) -> Result<B::GraphicsPipeline, Error> {
        let shader_modules = Self::create_shader_modules(&device, set)?;
        let result = {
            let shaders = Self::create_graphics_shader_set(&shader_modules)?;
            let rasterizer = Self::create_rasterizer();

            Self::create_pipeline(
                &device,
                &render_pass,
                layout,
                shaders,
                rasterizer,
            )?
        };

        Self::destroy_shader_modules(&device, shader_modules);

        Ok(result)
    }

    pub fn pipeline(&self) -> &B::GraphicsPipeline {
        &self.pipeline
    }

    fn create_pipeline(
        device: &D,
        render_pass: &B::RenderPass,
        layout: &B::PipelineLayout,
        shaders: GraphicsShaderSet<B>,
        rasterizer: Rasterizer
    ) -> Result<B::GraphicsPipeline, Error> {
        let input_assembler = InputAssemblerDesc::new(Primitive::TriangleList);

        let vertex_buffers: Vec<VertexBufferDesc> = vec![VertexBufferDesc {
            binding: 0,
            stride: V::stride() as u32,
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
                color: BlendOp::ALPHA,
                alpha: BlendOp::ADD,
            };
            BlendDesc {
                logic_op: Some(LogicOp::Copy),
                targets: vec![ColorBlendDesc(ColorMask::ALL, blend_state)],
            }
        };

        let baked_states = BakedStates {
            viewport: None,
            scissor: None,
            blend_color: None,
            depth_bounds: None,
        };

        let desc = GraphicsPipelineDesc {
            shaders,
            rasterizer,
            vertex_buffers,
            attributes: V::attributes(),
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

    fn create_shader_modules(
        device: &D,
        set: &ShaderSet,
    ) -> Result<[Option<B::ShaderModule>; 5], Error> {
        Ok([
            Some(unsafe { device.create_shader_module(set.vertex.spirv)? }),
            Self::map_to_shader_module(device, &set.hull)?,
            Self::map_to_shader_module(device, &set.domain)?,
            Self::map_to_shader_module(device, &set.geometry)?,
            Self::map_to_shader_module(device, &set.fragment)?,
        ])
    }

    fn destroy_shader_modules(device: &D, mut shader_modules: [Option<B::ShaderModule>; 5]) {
        for shader in &mut shader_modules {
            if let Some(s) = shader.take() {
                unsafe { device.destroy_shader_module(s) }
            }
        }
    }

    fn create_graphics_shader_set(
        modules: &[Option<B::ShaderModule>],
    ) -> Result<GraphicsShaderSet<B>, Error> {
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
        device: &D,
        desc: &Option<ShaderDescription>,
    ) -> Result<Option<B::ShaderModule>, Error> {
        match desc {
            Some(d) => Ok(Some(unsafe { device.create_shader_module(d.spirv)? })),
            None => Ok(None),
        }
    }

    fn create_shader_entry_point(shader_module: &Option<B::ShaderModule>) -> Option<EntryPoint<B>> {
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

impl<V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Drop
    for PipelineBundle<V, A, B, D, I>
{
    fn drop(&mut self) {
        use core::ptr::read;

        info!("{}", "Dropping Pipeline".red());

        let device = &self.state.device();
        let pipeline = &self.pipeline;

        unsafe {
            device.destroy_graphics_pipeline(ManuallyDrop::into_inner(read(pipeline)));
        }
    }
}

impl<V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Debug
    for PipelineBundle<V, A, B, D, I>
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self.pipeline_layout)?;
        write!(f, "{}", self.state)?;
        Ok(())
    }
}
