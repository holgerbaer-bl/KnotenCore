//! Sprint 184: Physics world extracted from the registry monolith.
//!
//! Retained-mode physics simulation: entity AABBs, collision detection,
//! and raycast-based mouse picking. Uses the global PHYSICS_WORLD.

use std::collections::HashMap;
use std::sync::Mutex;

/// Global physics world: entity_id → EntityPhysics
pub static PHYSICS_WORLD: Mutex<Option<HashMap<usize, EntityPhysics>>> = Mutex::new(None);

#[derive(Clone)]
pub struct EntityPhysics {
    pub base_aabb: crate::math::AABB,
    pub world_aabb: crate::math::AABB,
    pub transform: glam::Mat4,
}

/// Returns true if the world AABBs of the two entities intersect.
pub fn registry_check_collision(id1: i64, id2: i64) -> bool {
    let guard = PHYSICS_WORLD.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(map) = guard.as_ref()
        && let (Some(e1), Some(e2)) = (map.get(&(id1 as usize)), map.get(&(id2 as usize)))
    {
        return e1.world_aabb.intersects(&e2.world_aabb);
    }
    false
}

/// Checks if the user clicked on an entity and returns its ID.
/// Returns -1 if no entity was clicked.
pub fn registry_get_clicked_entity(window_handle: i64) -> i64 {
    if window_handle < 0 {
        return -1;
    }
    let input = super::registry::with_registry(|reg| {
        if let Some(entry) = reg.get(&(window_handle as usize))
            && let super::registry::NativeHandle::Window(proxy) = &entry.handle
        {
            return Some(proxy.input.clone());
        }
        None
    });

    if let Some(input_arc) = input {
        let mut state = input_arc.lock().unwrap_or_else(|e| e.into_inner());
        if state.mouse_clicked {
            state.mouse_clicked = false; // Consume click

            let vp = glam::Mat4::from_cols_array_2d(&state.view_proj);
            let inv_vp = vp.inverse();

            let ndc_x = (state.mouse_x / state.window_width) * 2.0 - 1.0;
            let ndc_y = 1.0 - (state.mouse_y / state.window_height) * 2.0;

            let clip_near = glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
            let clip_far = glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

            let mut world_near = inv_vp * clip_near;
            world_near /= world_near.w;

            let mut world_far = inv_vp * clip_far;
            world_far /= world_far.w;

            let ray_origin = world_near.truncate();
            let ray_dir = (world_far.truncate() - ray_origin).normalize();

            let guard = PHYSICS_WORLD.lock().unwrap_or_else(|e| e.into_inner());
            let mut hit_idx: i64 = -1;
            let mut t_min = f32::MAX;
            if let Some(map) = guard.as_ref() {
                for (&id, phys) in map.iter() {
                    if let Some(t) = phys.world_aabb.intersect_ray(ray_origin, ray_dir)
                        && t < t_min
                    {
                        t_min = t;
                        hit_idx = id as i64;
                    }
                }
            }
            return hit_idx;
        }
    }
    -1
}
