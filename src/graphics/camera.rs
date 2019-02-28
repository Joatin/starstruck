use std::fmt::Debug;
use vek::mat::Mat4;

pub trait Camera: Debug {
    fn projection_view(&self) -> Mat4<f32>;
}
