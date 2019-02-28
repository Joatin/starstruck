use crate::context::Context;
use crate::graphics::Camera;
use vek::geom::FrustumPlanes;
use vek::mat::Mat4;
use vek::vec::Vec3;
use winit::VirtualKeyCode;

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

    pub fn new() -> Self {
        Self {
            perspective_projection: Mat4::perspective_lh_zo(f32::to_radians(50.0), 1.0, 0.1, 100.0),
            orthogonal_projection: Mat4::identity(),
            position: Vec3::zero(),
            pitch_deg: 0.0,
            yaw_deg: 0.0,
            use_perspective: true,
        }
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

    pub fn set_position(&mut self, pos: Vec3<f32>) {
        self.position = pos;
    }

    pub fn update_from_context(&mut self, context: &Context) {
        let input = context.input();

        // Update projections
        let area = context.render_area();
        let ratio = area.width as f32 / area.height as f32;
        self.perspective_projection =
            Mat4::perspective_lh_zo(f32::to_radians(50.0), ratio, 0.1, 100.0);

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
        let mut move_vector =
            input.keys_held
                .iter()
                .fold(Vec3 {
                    x: 0.0_f32,
                    y: 0.0_f32,
                    z: 0.0_f32
                }, |vec, key| match *key {
                    VirtualKeyCode::W => vec + forward,
                    VirtualKeyCode::S => vec - forward,
                    VirtualKeyCode::A => vec - cross_normalized,
                    VirtualKeyCode::D => vec + cross_normalized,
                    VirtualKeyCode::Space => vec + up,
                    VirtualKeyCode::LShift => vec - up,
                    _ => vec,
                });

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
