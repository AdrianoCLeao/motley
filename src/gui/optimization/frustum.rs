use glam::{Vec3, Vec4, Mat4};

pub struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    pub fn from_view_projection_matrix(view_proj: &Mat4) -> Self {
        let m = *view_proj;

        Self {
            planes: [
                Vec4::new(
                    m.w_axis.x + m.x_axis.x,
                    m.w_axis.y + m.x_axis.y,
                    m.w_axis.z + m.x_axis.z,
                    m.w_axis.w + m.x_axis.w,
                ),
                Vec4::new(
                    m.w_axis.x - m.x_axis.x,
                    m.w_axis.y - m.x_axis.y,
                    m.w_axis.z - m.x_axis.z,
                    m.w_axis.w - m.x_axis.w,
                ), 
                Vec4::new(
                    m.w_axis.x + m.y_axis.x,
                    m.w_axis.y + m.y_axis.y,
                    m.w_axis.z + m.y_axis.z,
                    m.w_axis.w + m.y_axis.w,
                ), 
                Vec4::new(
                    m.w_axis.x - m.y_axis.x,
                    m.w_axis.y - m.y_axis.y,
                    m.w_axis.z - m.y_axis.z,
                    m.w_axis.w - m.y_axis.w,
                ), 
                Vec4::new(
                    m.w_axis.x + m.z_axis.x,
                    m.w_axis.y + m.z_axis.y,
                    m.w_axis.z + m.z_axis.z,
                    m.w_axis.w + m.z_axis.w,
                ), 
                Vec4::new(
                    m.w_axis.x - m.z_axis.x,
                    m.w_axis.y - m.z_axis.y,
                    m.w_axis.z - m.z_axis.z,
                    m.w_axis.w - m.z_axis.w,
                ), 
            ],
        }
    }

    pub fn is_box_in_frustum(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            let p = Vec3::new(
                if plane.x > 0.0 { max.x } else { min.x },
                if plane.y > 0.0 { max.y } else { min.y },
                if plane.z > 0.0 { max.z } else { min.z },
            );

            if plane.x * p.x + plane.y * p.y + plane.z * p.z + plane.w <= 0.0 {
                return false;
            }
        }
        true
    }
}
