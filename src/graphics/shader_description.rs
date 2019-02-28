#[derive(Debug, Clone)]
pub struct ShaderDescription {
    pub spirv: &'static [u8],
    pub constant_byte_size: usize,
}
