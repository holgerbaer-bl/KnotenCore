#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl AABB {
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

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

    pub fn intersect_ray(&self, ray_origin: glam::Vec3, ray_dir: glam::Vec3) -> Option<f32> {
        let inv_dir = glam::Vec3::new(1.0 / ray_dir.x, 1.0 / ray_dir.y, 1.0 / ray_dir.z);
        let t0 = (glam::Vec3::from_array(self.min) - ray_origin) * inv_dir;
        let t1 = (glam::Vec3::from_array(self.max) - ray_origin) * inv_dir;

        let tmin = t0.min(t1);
        let tmax = t0.max(t1);

        let t_min_val = tmin.max_element();
        let t_max_val = tmax.min_element();

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
}
