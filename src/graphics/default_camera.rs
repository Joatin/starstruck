use crate::graphics::Camera;
use vek::mat::Mat4;
use vek::vec::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct DefaultCamera {
    perspective_projection: Mat4<f32>,
    orthogonal_projection: Mat4<f32>,
    position: Vec3<f32>,
    pitch_deg: f32,
    yaw_deg: f32,
}

impl DefaultCamera {
    pub fn new() -> Self {
        Self {
            perspective_projection: Mat4::identity(),
            orthogonal_projection: Mat4::identity(),
            position: Vec3::zero(),
            pitch_deg: 0.0,
            yaw_deg: 0.0
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

    pub fn update_orientation(&mut self, d_pitch_deg: f32, d_yaw_deg: f32) {
        self.pitch_deg = (self.pitch_deg + d_pitch_deg).max(-89.0).min(89.0);
        self.yaw_deg = (self.yaw_deg + d_yaw_deg) % 360.0;
        trace!("New view pith {:?}, yew {:?}", self.pitch_deg, self.yaw_deg);
    }
}

impl Camera for DefaultCamera {
    fn projection_view(&self) -> Mat4<f32> {
        let view = Mat4::<f32>::look_at(
            self.position,
            self.position + self.make_front(),
            Vec3::<f32>::down()
        );

        self.perspective_projection * view
    }
}