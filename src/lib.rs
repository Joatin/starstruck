//! # Starstruck
//!
//! `Starstruck` Is a library that helps you make 3D applications with ease. This is primarily a
//! educational project. Its focus is to provide an easy user experience while still maintaining
//! high performance

#[cfg(windows)]
extern crate gfx_backend_dx12 as backend;
#[cfg(target_os = "macos")]
extern crate gfx_backend_metal as backend;
#[cfg(all(unix, not(target_os = "macos")))]
extern crate gfx_backend_vulkan as backend;

#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;

mod context;
mod internal;
mod setup_context;
mod starstruck;

pub mod camera;
pub mod errors;
pub mod graphics;
pub mod input;
pub mod menu;
pub mod primitive;

pub use self::context::*;
pub use self::setup_context::*;
pub use self::starstruck::Starstruck;
