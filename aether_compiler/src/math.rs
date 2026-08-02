#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl AABB {
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    #[cfg(feature = "ui")]
    pub fn transform(&self, transform: &glam::Mat4) -> Self {
        let mut min = glam::Vec3::splat(f32::INFINITY);
        let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
        let corners = [
            glam::Vec3::new(self.min[0], self.min[1], self.min[2]),
            glam::Vec3::new(self.max[0], self.min[1], self.min[2]),
            glam::Vec3::new(self.min[0], self.max[1], self.min[2]),
            glam::Vec3::new(self.max[0], self.max[1], self.min[2]),
            glam::Vec3::new(self.min[0], self.min[1], self.max[2]),
            glam::Vec3::new(self.max[0], self.min[1], self.max[2]),
            glam::Vec3::new(self.min[0], self.max[1], self.max[2]),
            glam::Vec3::new(self.max[0], self.max[1], self.max[2]),
        ];
        for corner in &corners {
            let transformed = (*transform * corner.extend(1.0)).truncate();
            min = min.min(transformed);
            max = max.max(transformed);
        }
        Self::new(min.into(), max.into())
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min[0] <= other.max[0]
            && self.max[0] >= other.min[0]
            && self.min[1] <= other.max[1]
            && self.max[1] >= other.min[1]
            && self.min[2] <= other.max[2]
            && self.max[2] >= other.min[2]
    }

    pub fn intersect_ray_coords(&self, ray_origin: [f32; 3], ray_dir: [f32; 3]) -> Option<f32> {
        let inv_x = 1.0 / ray_dir[0];
        let inv_y = 1.0 / ray_dir[1];
        let inv_z = 1.0 / ray_dir[2];

        let t0_x = (self.min[0] - ray_origin[0]) * inv_x;
        let t1_x = (self.max[0] - ray_origin[0]) * inv_x;
        let tmin_x = t0_x.min(t1_x);
        let tmax_x = t0_x.max(t1_x);

        let t0_y = (self.min[1] - ray_origin[1]) * inv_y;
        let t1_y = (self.max[1] - ray_origin[1]) * inv_y;
        let tmin_y = t0_y.min(t1_y);
        let tmax_y = t0_y.max(t1_y);

        let t0_z = (self.min[2] - ray_origin[2]) * inv_z;
        let t1_z = (self.max[2] - ray_origin[2]) * inv_z;
        let tmin_z = t0_z.min(t1_z);
        let tmax_z = t0_z.max(t1_z);

        let t_min_val = tmin_x.max(tmin_y).max(tmin_z);
        let t_max_val = tmax_x.min(tmax_y).min(tmax_z);

        if t_max_val >= t_min_val && t_max_val >= 0.0 {
            if t_min_val < 0.0 {
                Some(t_max_val)
            } else {
                Some(t_min_val)
            }
        } else {
            None
        }
    }

    #[cfg(feature = "ui")]
    pub fn intersect_ray(&self, ray_origin: glam::Vec3, ray_dir: glam::Vec3) -> Option<f32> {
        self.intersect_ray_coords(ray_origin.into(), ray_dir.into())
    }
}
