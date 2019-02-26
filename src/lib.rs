
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


mod starstruck;
mod internal;

pub mod context;
pub mod input;
pub mod setup_context;
pub mod primitive;
pub mod graphics;
pub mod errors;

pub use self::starstruck::Starstruck;


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
