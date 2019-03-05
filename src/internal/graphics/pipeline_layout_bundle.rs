use crate::graphics::ShaderSet;
use crate::internal::graphics::GraphicsState;
use colored::*;
use failure::Error;
use gfx_hal::pso::Descriptor;
use gfx_hal::pso::DescriptorArrayIndex;
use gfx_hal::pso::DescriptorBinding;
use gfx_hal::pso::DescriptorPool;
use gfx_hal::pso::DescriptorRangeDesc;
use gfx_hal::pso::DescriptorSetLayoutBinding;
use gfx_hal::pso::DescriptorSetWrite;
use gfx_hal::pso::DescriptorType;
use gfx_hal::pso::ShaderStageFlags;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::mem::ManuallyDrop;
use std::ops::Range;
use std::sync::Arc;

pub struct PipelineLayoutBundle<B: Backend, D: Device<B>, I: Instance<Backend = B>> {
    descriptor_layouts: Vec<B::DescriptorSetLayout>,
    layout: ManuallyDrop<B::PipelineLayout>,
    push_constants: Vec<(ShaderStageFlags, Range<u32>)>,
    descriptor_pool: ManuallyDrop<B::DescriptorPool>,
    descriptor_set: B::DescriptorSet,
    state: Arc<GraphicsState<B, D, I>>,
}

impl<B: Backend, D: Device<B>, I: Instance<Backend = B>> PipelineLayoutBundle<B, D, I> {
    pub fn new(state: Arc<GraphicsState<B, D, I>>, set: &ShaderSet) -> Result<Self, Error> {
        let mut bindings = Vec::<DescriptorSetLayoutBinding>::new();
        let mut range = HashMap::<DescriptorType, DescriptorRangeDesc>::new();
        for (binding, ty, count) in &set.vertex.bindings {
            bindings.push(DescriptorSetLayoutBinding {
                binding: *binding,
                ty: *ty,
                count: *count,
                stage_flags: ShaderStageFlags::VERTEX,
                immutable_samplers: false,
            });
            if let Some(ran) = range.get_mut(ty) {
                ran.count += 1;
            } else {
                range.insert(*ty, DescriptorRangeDesc { ty: *ty, count: 1 });
            }
        }
        if let Some(hull) = set.hull.as_ref() {
            for (binding, ty, count) in &hull.bindings {
                bindings.push(DescriptorSetLayoutBinding {
                    binding: *binding,
                    ty: *ty,
                    count: *count,
                    stage_flags: ShaderStageFlags::HULL,
                    immutable_samplers: false,
                });
                if let Some(ran) = range.get_mut(ty) {
                    ran.count += 1;
                } else {
                    range.insert(*ty, DescriptorRangeDesc { ty: *ty, count: 1 });
                }
            }
        }
        if let Some(domain) = set.domain.as_ref() {
            for (binding, ty, count) in &domain.bindings {
                bindings.push(DescriptorSetLayoutBinding {
                    binding: *binding,
                    ty: *ty,
                    count: *count,
                    stage_flags: ShaderStageFlags::DOMAIN,
                    immutable_samplers: false,
                });
                if let Some(ran) = range.get_mut(ty) {
                    ran.count += 1;
                } else {
                    range.insert(*ty, DescriptorRangeDesc { ty: *ty, count: 1 });
                }
            }
        }
        if let Some(fragment) = set.fragment.as_ref() {
            for (binding, ty, count) in &fragment.bindings {
                bindings.push(DescriptorSetLayoutBinding {
                    binding: *binding,
                    ty: *ty,
                    count: *count,
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    immutable_samplers: false,
                });
                if let Some(ran) = range.get_mut(ty) {
                    ran.count += 1;
                } else {
                    range.insert(*ty, DescriptorRangeDesc { ty: *ty, count: 1 });
                }
            }
        }
        if let Some(geometry) = set.geometry.as_ref() {
            for (binding, ty, count) in &geometry.bindings {
                bindings.push(DescriptorSetLayoutBinding {
                    binding: *binding,
                    ty: *ty,
                    count: *count,
                    stage_flags: ShaderStageFlags::GEOMETRY,
                    immutable_samplers: false,
                });
                if let Some(ran) = range.get_mut(ty) {
                    ran.count += 1;
                } else {
                    range.insert(*ty, DescriptorRangeDesc { ty: *ty, count: 1 });
                }
            }
        }

        let immutable_samplers = Vec::<B::Sampler>::new();
        let descriptor_layouts = vec![unsafe {
            state
                .device()
                .create_descriptor_set_layout(bindings, immutable_samplers)?
        }];

        let mut push_constants = Vec::with_capacity(2);
        push_constants.push((ShaderStageFlags::VERTEX, 0..set.vertex.push_constant_floats));
        if let Some(hull) = set.hull.as_ref() {
            push_constants.push((ShaderStageFlags::HULL, 0..hull.push_constant_floats));
        }
        if let Some(domain) = set.domain.as_ref() {
            push_constants.push((ShaderStageFlags::DOMAIN, 0..domain.push_constant_floats));
        }
        if let Some(fragment) = set.fragment.as_ref() {
            push_constants.push((ShaderStageFlags::FRAGMENT, 0..fragment.push_constant_floats));
        }
        if let Some(geometry) = set.geometry.as_ref() {
            push_constants.push((ShaderStageFlags::GEOMETRY, 0..geometry.push_constant_floats));
        }

        let layout = unsafe {
            state
                .device()
                .create_pipeline_layout(&descriptor_layouts, &push_constants)?
        };

        let mut descriptor_pool =
            unsafe { state.device().create_descriptor_pool(1, range.values()) }?;

        let descriptor_set = unsafe { descriptor_pool.allocate_set(&descriptor_layouts[0])? };

        Ok(Self {
            descriptor_layouts,
            descriptor_set,
            descriptor_pool: ManuallyDrop::new(descriptor_pool),
            layout: ManuallyDrop::new(layout),
            push_constants,
            state,
        })
    }

    pub fn bind_assets(
        &self,
        descriptors: Vec<(DescriptorBinding, DescriptorArrayIndex, Descriptor<B>)>,
    ) {
        unsafe {
            let result: Vec<DescriptorSetWrite<B, _>> = descriptors
                .into_iter()
                .map(|(binding, array_offset, desc)| DescriptorSetWrite {
                    set: &self.descriptor_set,
                    binding,
                    array_offset,
                    descriptors: Some(desc),
                })
                .collect();
            self.state.device().write_descriptor_sets(result);
        }
    }

    pub fn layout(&self) -> &B::PipelineLayout {
        &self.layout
    }

    pub fn descriptor_set(&self) -> &B::DescriptorSet {
        &self.descriptor_set
    }
}

impl<B: Backend, D: Device<B>, I: Instance<Backend = B>> Drop for PipelineLayoutBundle<B, D, I> {
    fn drop(&mut self) {
        use core::ptr::read;

        info!("{}", "Dropping Pipeline Layout".red());

        let device = &self.state.device();
        let layout = &self.layout;
        let descriptor_layouts = &mut self.descriptor_layouts;
        let descriptor_pool = &mut self.descriptor_pool;

        unsafe {
            descriptor_pool.reset();
            device.destroy_descriptor_pool(ManuallyDrop::into_inner(read(descriptor_pool)));
            device.destroy_pipeline_layout(ManuallyDrop::into_inner(read(layout)));
            for item in descriptor_layouts.drain(1..) {
                device.destroy_descriptor_set_layout(item);
            }
        }
    }
}

impl<B: Backend, D: Device<B>, I: Instance<Backend = B>> Debug for PipelineLayoutBundle<B, D, I> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self.push_constants)?;
        write!(f, "{:?}", self.descriptor_set)?;
        write!(f, "{}", self.state)?;
        Ok(())
    }
}
