use crate::camera::Camera;
use crate::context::Context;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;
use vek::geom::FrustumPlanes;
use vek::mat::Mat4;
use vek::vec::Vec3;
use winit::VirtualKeyCode;

/// This camera implements the Camera trait. This camera needs to be updated each frame from the
/// Context. This camera can be freely navigated in the world space. You can also toggle the
/// projection. The key bindings are as follows:
///
/// * UP: Space
/// * DOWN: LShift
/// * FORWARD: W
/// * BACKWARD: S
/// * LEFT: A
/// * RIGHT: D
/// * ROTATE_UP: ArrowUp
/// * ROTATE_DOWN: ArrowDown
/// * ROTATE_LEFT: ArrowLeft
/// * ROTATE_RIGHT: ArrowRight
/// * TOGGlE_PERSPECTIVE: F1
///
/// Observe that this camera is only intended for early scaffolding or debugging purposes. You do
/// not want this in your final app
#[derive(Debug, Clone, Copy)]
pub struct DebugCamera {
    perspective_projection: Mat4<f32>,
    orthogonal_projection: Mat4<f32>,
    position: Vec3<f32>,
    pitch_deg: f32,
    yaw_deg: f32,
    use_perspective: bool,
}

impl DebugCamera {
    const ZOOM: f32 = 0.005;

    /// Constructs a new debug camera
    pub fn new() -> Self {
        Self {
            perspective_projection: Mat4::perspective_lh_zo(
                f32::to_radians(70.0),
                1.0,
                0.01,
                100.0,
            ),
            orthogonal_projection: Mat4::identity(),
            position: Vec3::zero(),
            pitch_deg: 0.0,
            yaw_deg: 0.0,
            use_perspective: true,
        }
    }

    /// Tells if the camera is using perspective projection, otherwise orthogonal
    pub fn is_perspective(&self) -> bool {
        self.use_perspective
    }

    fn make_front(&self) -> Vec3<f32> {
        let pitch_rad = f32::to_radians(self.pitch_deg);
        let yaw_rad = f32::to_radians(self.yaw_deg);
        Vec3 {
            x: yaw_rad.sin() * pitch_rad.cos(),
            y: pitch_rad.sin(),
            z: yaw_rad.cos() * pitch_rad.cos(),
        }
    }

    fn update_orientation(&mut self, d_pitch_deg: f32, d_yaw_deg: f32) {
        self.pitch_deg = (self.pitch_deg + d_pitch_deg).max(-89.0).min(89.0);
        self.yaw_deg = (self.yaw_deg + d_yaw_deg) % 360.0;
    }

    /// Sets the cameras position in the world
    pub fn set_position(&mut self, pos: Vec3<f32>) {
        self.position = pos;
    }

    /// Updates the camera from the context. This makes sure the the projects aspects ratios are
    /// correct with the windows size. It also takes care of moving the camera based on the user
    /// input.
    ///
    /// # Examples
    ///
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use starstruck::StarstruckBuilder;
    /// use starstruck::camera::DebugCamera;
    ///
    /// let starstruck = StarstruckBuilder::new_with_setup(|_| Ok(DebugCamera::new()))
    ///     .with_render_callback(|(camera, context)| {
    ///         camera.update_from_context(context);
    ///         Ok(())
    ///     })
    ///     .init()?;
    /// # Ok(())
    /// # }
    ///
    /// ```
    pub fn update_from_context<B: Backend, D: Device<B>, I: Instance<Backend = B>>(
        &mut self,
        context: &Context<B, D, I>,
    ) {
        let input = context.input();

        // Update projections
        let area = context.render_area();
        let ratio = area.width as f32 / area.height as f32;
        self.perspective_projection =
            Mat4::perspective_lh_zo(f32::to_radians(70.0), ratio, 0.1, 100.0);

        let o_height = (area.height as f32 / 2.0) * Self::ZOOM;
        let o_width = (area.width as f32 / 2.0) * Self::ZOOM;
        self.orthogonal_projection = Mat4::orthographic_lh_zo(FrustumPlanes {
            left: o_width * -1.0,
            right: o_width,
            bottom: o_height * -1.,
            top: o_height,
            near: 0.,
            far: 100.,
        });

        if input.keys_clicked.contains(&VirtualKeyCode::F1) {
            self.use_perspective = !self.use_perspective;
        }

        let up = Vec3::up();
        let forward = self.make_front();
        let cross_normalized = Vec3::cross(forward, up).normalized();
        let mut move_vector = input.keys_held.iter().fold(
            Vec3 {
                x: 0.0_f32,
                y: 0.0_f32,
                z: 0.0_f32,
            },
            |vec, key| match *key {
                VirtualKeyCode::W => vec + forward,
                VirtualKeyCode::S => vec - forward,
                VirtualKeyCode::A => vec - cross_normalized,
                VirtualKeyCode::D => vec + cross_normalized,
                VirtualKeyCode::Space => vec + up,
                VirtualKeyCode::LShift => vec - up,
                _ => vec,
            },
        );

        if move_vector != Vec3::zero() {
            move_vector = move_vector.normalized();
            self.position += move_vector * 0.01;
        }

        if input.keys_held.contains(&VirtualKeyCode::Up) {
            self.update_orientation(0.5, 0.0);
        }

        if input.keys_held.contains(&VirtualKeyCode::Down) {
            self.update_orientation(-0.5, 0.0);
        }

        if input.keys_held.contains(&VirtualKeyCode::Left) {
            self.update_orientation(0.0, 0.5);
        }

        if input.keys_held.contains(&VirtualKeyCode::Right) {
            self.update_orientation(0.0, -0.5);
        }
    }
}

impl Default for DebugCamera {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera for DebugCamera {
    fn projection_view(&self) -> Mat4<f32> {
        let view = Mat4::<f32>::look_at(
            self.position,
            self.position + self.make_front(),
            Vec3::<f32>::down(),
        );

        if self.use_perspective {
            self.perspective_projection * view
        } else {
            self.orthogonal_projection * view
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::camera::DebugCamera;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_should_use_perspective_by_default() {
        let camera = DebugCamera::new();

        assert_eq!(true, camera.is_perspective())
    }
}
