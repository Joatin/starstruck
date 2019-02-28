use crate::graphics::ShaderDescription;

#[derive(Debug, Clone)]
pub struct ShaderSet {
    pub vertex: ShaderDescription,
    pub hull: Option<ShaderDescription>,
    pub domain: Option<ShaderDescription>,
    pub geometry: Option<ShaderDescription>,
    pub fragment: Option<ShaderDescription>,
}
