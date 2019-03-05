use gfx_hal::pso::DescriptorArrayIndex;
use gfx_hal::pso::DescriptorBinding;
use gfx_hal::pso::DescriptorType;

#[derive(Debug, Clone)]
pub struct ShaderDescription {
    pub spirv: &'static [u8],
    pub push_constant_floats: u32,
    pub bindings: Vec<(DescriptorBinding, DescriptorType, DescriptorArrayIndex)>,
}
