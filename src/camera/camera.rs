use std::fmt::Debug;
use vek::mat::Mat4;

/// Common trait for all types of cameras
///
/// While this project come with a few implementations of this trait, the common approach is to
/// implement this yourself so that the camera follows your main character
pub trait Camera: Debug {
    /// Returns the calculated projection view matrix
    ///
    /// This a combination of the cameras position and rotation in the world as well as the cameras
    /// projection, typically orthogonal or perspective
    fn projection_view(&self) -> Mat4<f32>;
}
