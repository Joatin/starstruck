//! This module contains everything related to camera and projection. Usually you use the module to
//! implement you own camera that determines how everything in your app will be rendered

mod camera;
mod debug_camera;

#[doc(inline)]
pub use self::camera::Camera;

#[doc(inline)]
pub use self::debug_camera::DebugCamera;
