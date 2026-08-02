//! Sprint 184: Scene graph extracted from the registry monolith.
//!
//! Retained-mode 3D scene graph: entity spawning, transforms, lighting,
//! camera, and GPU lifecycle. Calls into geometry (mesh generation) and
//! physics (AABB registration).

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(feature = "ui")]
use winit::window::Window as WinitWindow;

use crate::natives::registry::{RenderCommand, send_render_command};

/// Retained-mode scene graph entity
pub struct SceneEntity {
    pub mesh_name: String,
    pub texture_id: usize,
    pub transform: glam::Mat4,
    /// Sprint 209: Dirty flag — set true on spawn/update, cleared after GPU upload
    pub is_dirty: bool,
}

/// Retained-mode dynamic point light
pub struct SceneLight {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}

/// Per-window WGPU + input + scene state
#[cfg(feature = "ui")]
pub struct RegistryWindowState {
    pub window: Arc<WinitWindow>,
    pub input: Arc<Mutex<crate::natives::registry::InputState>>,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,
    pub config: wgpu::SurfaceConfiguration,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub pipeline: wgpu::RenderPipeline,
    // Sprint 206: Instance buffer for hardware instancing (1024 instances)
    pub instance_buffer: wgpu::Buffer,
    pub width: u32,
    pub height: u32,
    pub clear_color: wgpu::Color,
    pub current_texture: Option<wgpu::SurfaceTexture>,
    pub current_view: Option<wgpu::TextureView>,
    pub encoder: Option<wgpu::CommandEncoder>,
    pub depth_texture_view: wgpu::TextureView,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub model_buffer: wgpu::Buffer,
    pub model_bind_group: wgpu::BindGroup,
    pub geometry_cache: std::collections::HashMap<String, super::geometry::CachedMesh>,
    pub texture_cache: std::collections::HashMap<usize, wgpu::BindGroup>,
    pub default_texture_bind_group: wgpu::BindGroup,
    pub commands: Vec<RenderCommand>,
    pub scene_graph: std::collections::HashMap<usize, SceneEntity>,
    pub lights: std::collections::HashMap<usize, SceneLight>,
    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
    pub ui_tree: Vec<knoten_core_types::ast::Node>,
    pub particle_pipeline: Option<wgpu::RenderPipeline>,
    pub particle_bgl: Option<wgpu::BindGroupLayout>,
}

#[cfg(not(feature = "ui"))]
pub struct RegistryWindowState;

pub(crate) static NEXT_ENTITY_ID: AtomicUsize = AtomicUsize::new(1);
pub(crate) static NEXT_LIGHT_ID: AtomicUsize = AtomicUsize::new(1);

pub(crate) static SENT_MESHES: std::sync::Mutex<Option<std::collections::HashSet<String>>> =
    std::sync::Mutex::new(None);

fn ensure_mesh_sent(
    mesh_name: &str,
    vertices: Vec<super::geometry::RegistryVertex>,
    indices: Vec<u32>,
) {
    let mut guard = SENT_MESHES.lock().unwrap_or_else(|e| e.into_inner());
    let sent = guard.get_or_insert_with(std::collections::HashSet::new);
    if !sent.contains(mesh_name) {
        send_render_command(RenderCommand::AddMesh {
            name: mesh_name.to_string(),
            vertices,
            indices,
        });
        sent.insert(mesh_name.to_string());
    }
}

#[allow(clippy::too_many_arguments)]
pub fn registry_spawn_cube(
    window_handle: i64,
    texture_handle: i64,
    w: f32,
    h: f32,
    d: f32,
    x: f32,
    y: f32,
    z: f32,
) -> i64 {
    if window_handle < 0 || texture_handle < 0 {
        return -1;
    }

    let mesh_name = "cube".to_string();
    let (vertices, indices) = super::geometry::generate_cube();
    ensure_mesh_sent(&mesh_name, vertices, indices);

    let entity_id = NEXT_ENTITY_ID.fetch_add(1, Ordering::Relaxed);

    let t = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
    let s = glam::Mat4::from_scale(glam::Vec3::new(w, h, d));
    let transform = t * s;

    let base_aabb = crate::math::AABB::new([-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]);
    let world_aabb = base_aabb.transform(&transform);
    let mut phys_guard = super::physics::PHYSICS_WORLD
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let phys_map = phys_guard.get_or_insert_with(std::collections::HashMap::new);
    phys_map.insert(
        entity_id,
        super::physics::EntityPhysics {
            base_aabb,
            world_aabb,
            transform,
        },
    );

    send_render_command(RenderCommand::SpawnEntity {
        window_id: window_handle as usize,
        entity_id,
        mesh_name,
        texture_id: texture_handle as usize,
        transform: transform.to_cols_array_2d(),
    });
    entity_id as i64
}

#[allow(clippy::too_many_arguments)]
pub fn registry_spawn_sphere(
    window_handle: i64,
    texture_handle: i64,
    radius: f32,
    rings: i32,
    sectors: i32,
    x: f32,
    y: f32,
    z: f32,
) -> i64 {
    if window_handle < 0 || texture_handle < 0 {
        return -1;
    }
    let rings = rings.max(3) as u32;
    let sectors = sectors.max(3) as u32;
    let mesh_name = format!("sphere_{}_{}", rings, sectors);

    let (vertices, indices) = super::geometry::generate_uv_sphere(rings, sectors);
    ensure_mesh_sent(&mesh_name, vertices, indices);

    let entity_id = NEXT_ENTITY_ID.fetch_add(1, Ordering::Relaxed);

    let t = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
    let s = glam::Mat4::from_scale(glam::Vec3::splat(radius));
    let transform = t * s;

    let base_aabb = crate::math::AABB::new([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);
    let world_aabb = base_aabb.transform(&transform);
    let mut phys_guard = super::physics::PHYSICS_WORLD
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let phys_map = phys_guard.get_or_insert_with(std::collections::HashMap::new);
    phys_map.insert(
        entity_id,
        super::physics::EntityPhysics {
            base_aabb,
            world_aabb,
            transform,
        },
    );

    send_render_command(RenderCommand::SpawnEntity {
        window_id: window_handle as usize,
        entity_id,
        mesh_name,
        texture_id: texture_handle as usize,
        transform: transform.to_cols_array_2d(),
    });
    entity_id as i64
}

#[allow(clippy::too_many_arguments)]
pub fn registry_spawn_cylinder(
    window_handle: i64,
    texture_handle: i64,
    radius: f32,
    height: f32,
    segments: i32,
    x: f32,
    y: f32,
    z: f32,
) -> i64 {
    if window_handle < 0 || texture_handle < 0 {
        return -1;
    }
    let segments = segments.max(3) as u32;
    let mesh_name = format!("cylinder_{}", segments);

    let (vertices, indices) = super::geometry::generate_cylinder(segments);
    ensure_mesh_sent(&mesh_name, vertices, indices);

    let entity_id = NEXT_ENTITY_ID.fetch_add(1, Ordering::Relaxed);

    let t = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
    let s = glam::Mat4::from_scale(glam::Vec3::new(radius, height, radius));
    let transform = t * s;

    let base_aabb = crate::math::AABB::new([-1.0, -0.5, -1.0], [1.0, 0.5, 1.0]);
    let world_aabb = base_aabb.transform(&transform);
    let mut phys_guard = super::physics::PHYSICS_WORLD
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let phys_map = phys_guard.get_or_insert_with(std::collections::HashMap::new);
    phys_map.insert(
        entity_id,
        super::physics::EntityPhysics {
            base_aabb,
            world_aabb,
            transform,
        },
    );

    send_render_command(RenderCommand::SpawnEntity {
        window_id: window_handle as usize,
        entity_id,
        mesh_name,
        texture_id: texture_handle as usize,
        transform: transform.to_cols_array_2d(),
    });
    entity_id as i64
}

pub fn registry_update_entity_transform(
    window_handle: i64,
    entity_handle: i64,
    x: f32,
    y: f32,
    z: f32,
) {
    if window_handle < 0 || entity_handle < 0 {
        return;
    }
    let t = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
    let mut final_transform = t;

    let mut phys_guard = super::physics::PHYSICS_WORLD
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(phys_map) = phys_guard.as_mut()
        && let Some(phys) = phys_map.get_mut(&(entity_handle as usize))
    {
        let old_scale = phys.transform.to_scale_rotation_translation().0;
        final_transform = t * glam::Mat4::from_scale(old_scale);
        phys.transform = final_transform;
        phys.world_aabb = phys.base_aabb.transform(&final_transform);
    }

    send_render_command(RenderCommand::UpdateEntityTransform {
        window_id: window_handle as usize,
        entity_id: entity_handle as usize,
        transform: final_transform.to_cols_array_2d(),
    });
}

pub fn registry_destroy_entity(window_handle: i64, entity_id: i64) {
    if window_handle < 0 || entity_id < 0 {
        eprintln!(
            "[FFI Safety] registry_destroy_entity rejected null handle: win={} entity={}",
            window_handle, entity_id
        );
        return;
    }
    let eid = entity_id as usize;

    let mut phys_guard = super::physics::PHYSICS_WORLD
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let phys_existed = if let Some(phys_map) = phys_guard.as_mut() {
        phys_map.remove(&eid).is_some()
    } else {
        false
    };
    drop(phys_guard);

    if !phys_existed {
        eprintln!(
            "[FFI Safety] registry_destroy_entity: entity {} already freed or never existed",
            eid
        );
    }

    send_render_command(RenderCommand::RemoveEntity {
        window_id: window_handle as usize,
        entity_id: eid,
    });
}

#[allow(clippy::too_many_arguments)]
pub fn registry_spawn_light(
    window_handle: i64,
    x: f32,
    y: f32,
    z: f32,
    r: f32,
    g: f32,
    b: f32,
    intensity: f32,
) -> i64 {
    if window_handle < 0 {
        return -1;
    }
    let light_id = NEXT_LIGHT_ID.fetch_add(1, Ordering::Relaxed);

    send_render_command(RenderCommand::SpawnLight {
        window_id: window_handle as usize,
        light_id,
        x,
        y,
        z,
        r,
        g,
        b,
        intensity,
    });
    light_id as i64
}

pub fn registry_update_light_position(
    window_handle: i64,
    light_handle: i64,
    x: f32,
    y: f32,
    z: f32,
) {
    if window_handle < 0 || light_handle < 0 {
        return;
    }
    send_render_command(RenderCommand::UpdateLightPosition {
        window_id: window_handle as usize,
        light_id: light_handle as usize,
        x,
        y,
        z,
    });
}

/// Sprint 86: Compute a perspective view-proj matrix and send it to all windows.
pub fn registry_set_camera(fov_degrees: f32, cam_x: f32, cam_y: f32, cam_z: f32) {
    registry_set_camera_for_window(0, fov_degrees, cam_x, cam_y, cam_z);
}

/// Sprint 86: Send a camera to a specific window (window_id = handle_id).
pub fn registry_set_camera_for_window(
    window_id: i64,
    fov_degrees: f32,
    cam_x: f32,
    cam_y: f32,
    cam_z: f32,
) {
    use glam::{Mat4, Vec3};
    let eye = Vec3::new(cam_x, cam_y, cam_z);
    let target = Vec3::ZERO;
    let up = Vec3::Y;
    let aspect = 16.0_f32 / 9.0_f32;
    let proj = Mat4::perspective_rh(fov_degrees.to_radians(), aspect, 0.1, 1000.0);
    let view = Mat4::look_at_rh(eye, target, up);
    let vp = proj * view;
    let vp_arr = vp.to_cols_array_2d();

    send_render_command(RenderCommand::SetCamera {
        window_id: window_id as usize,
        view_proj: vp_arr,
    });
}
