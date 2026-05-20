use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use winit::window::Window as WinitWindow;

use std::collections::HashSet;
use winit::keyboard::KeyCode;

use std::sync::atomic::AtomicBool;

pub static GLOBAL_KEYS: [AtomicBool; 256] = [const { AtomicBool::new(false) }; 256];

pub struct InputState {
    pub keys: HashSet<KeyCode>,
    pub mouse_dx: f32,
    pub mouse_dy: f32,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_left_down: bool,
    pub mouse_clicked: bool,
    pub view_proj: [[f32; 4]; 4],
    pub window_width: f32,
    pub window_height: f32,
    pub last_char: u32,
}

pub enum RenderCommand {
    CreateWindow {
        id: usize,
        title: String,
        width: u32,
        height: u32,
        input: Arc<Mutex<InputState>>,
    },
    SpawnEntity {
        window_id: usize,
        entity_id: usize,
        mesh_name: String,
        texture_id: usize,
        transform: glam::Mat4,
    },
    UpdateEntityTransform {
        window_id: usize,
        entity_id: usize,
        transform: glam::Mat4,
    },
    /// Sprint 86: send a camera view-projection matrix to a specific window.
    SetCamera {
        window_id: usize,
        view_proj: [[f32; 4]; 4],
    },
    /// Sprint 167: spawn a dynamic point light into the per-window light registry.
    SpawnLight {
        window_id: usize,
        light_id: usize,
        x: f32,
        y: f32,
        z: f32,
        r: f32,
        g: f32,
        b: f32,
        intensity: f32,
    },
    /// Sprint 167: update the position of an existing point light.
    UpdateLightPosition {
        window_id: usize,
        light_id: usize,
        x: f32,
        y: f32,
        z: f32,
    },
    UpdateWindow(usize),
    UpdateUI {
        window_id: usize,
        nodes: Vec<crate::ast::Node>,
    },
    CloseWindow(usize),
    AddMesh {
        name: String,
        vertices: Vec<RegistryVertex>,
        indices: Vec<u32>,
    },
    LoadTexture {
        id: usize,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    LoadComputeShader {
        id: usize,
        source: String,
    },
    DispatchCompute {
        shader_id: usize,
        x: u32,
        y: u32,
        z: u32,
        inputs: Vec<crate::executor::RelType>,
    },
    RemoveEntity {
        window_id: usize,
        entity_id: usize,
    },
    ExitEventLoop,
}

pub fn exit_event_loop() {
    send_render_command(RenderCommand::ExitEventLoop);
}

static RENDER_TX: Mutex<Option<winit::event_loop::EventLoopProxy<RenderCommand>>> =
    Mutex::new(None);
static SENT_MESHES: Mutex<Option<HashSet<String>>> = Mutex::new(None);

pub static AUDIO_STATE: Mutex<Option<crate::audio::AudioManager>> = Mutex::new(None);

pub fn init_audio_state() {
    let mut guard = AUDIO_STATE.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none()
        && let Ok(manager) = crate::audio::AudioManager::new()
    {
        *guard = Some(manager);
    }
}

pub fn set_render_channel(tx: winit::event_loop::EventLoopProxy<RenderCommand>) {
    let mut guard = RENDER_TX.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(tx);
}

pub fn send_render_command(cmd: RenderCommand) {
    let guard = RENDER_TX.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(tx) = guard.as_ref() {
        let _ = tx.send_event(cmd);
    }
}

/// Sprint 162: Send a UI tree to a specific window for retained-mode rendering.
/// The window ID must match a window previously created with `registry_create_window`.
pub fn send_ui_nodes_to(window_id: usize, nodes: Vec<crate::ast::Node>) {
    send_render_command(RenderCommand::UpdateUI { window_id, nodes });
}

/// Legacy helper: broadcasts to window 1 (single-window scripts).
pub fn send_ui_nodes(nodes: Vec<crate::ast::Node>) {
    send_ui_nodes_to(1, nodes);
}

/// Sprint 162: Set the retained UI tree for a window.
/// Accepts a window handle (i64) and a vector of AST nodes.
/// The render thread will autonomously draw this tree at 60 FPS.
pub fn registry_ui_set(window_handle: i64, nodes: Vec<crate::ast::Node>) {
    if window_handle < 0 {
        return;
    }
    send_ui_nodes_to(window_handle as usize, nodes);
}

/// Sprint 162: Poll (and clear) the clicked state for a UI button.
/// Returns `true` once per click event; clears the flag after reading.
pub fn registry_ui_poll_button(label: String) -> bool {
    crate::natives::ui::ui_button_poll(&label)
}

/// Sprint 162: Read the current text from a keyed UITextInput widget.
/// `key` is the seed string used when the UITextInput was first defined.
pub fn registry_ui_read_text(key: String) -> String {
    crate::natives::ui::ui_text_read(&key)
}

// Proxy for a Window to be used by the background executor.
pub struct WindowProxy {
    pub id: usize,
    pub input: Arc<Mutex<InputState>>,
}

// Retained-Mode Physics Store
pub struct EntityPhysics {
    pub base_aabb: crate::math::AABB,
    pub world_aabb: crate::math::AABB,
    pub transform: glam::Mat4,
}

pub static PHYSICS_WORLD: std::sync::Mutex<Option<HashMap<usize, EntityPhysics>>> =
    std::sync::Mutex::new(None);
pub static TEXTURE_ID_COUNTER: std::sync::Mutex<usize> = std::sync::Mutex::new(1); // 0 is reserved for default

unsafe impl Send for WindowProxy {}
unsafe impl Sync for WindowProxy {}

pub struct SceneEntity {
    pub mesh_name: String,
    pub texture_id: usize,
    pub transform: glam::Mat4,
}

/// Sprint 167: A dynamic point light in the scene.
pub struct SceneLight {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}

pub struct RegistryWindowState {
    pub window: Arc<WinitWindow>,
    pub input: Arc<Mutex<InputState>>,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,
    /// Sprint 86: store the full config so resize can just mutate width/height and reconfigure.
    pub config: wgpu::SurfaceConfiguration,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub pipeline: wgpu::RenderPipeline,
    pub width: u32,
    pub height: u32,
    pub clear_color: wgpu::Color,
    pub current_texture: Option<wgpu::SurfaceTexture>,
    pub current_view: Option<wgpu::TextureView>,
    pub encoder: Option<wgpu::CommandEncoder>,
    // 3D Resources
    pub depth_texture_view: wgpu::TextureView,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub model_buffer: wgpu::Buffer,
    pub model_bind_group: wgpu::BindGroup,
    pub geometry_cache: HashMap<String, CachedMesh>,
    pub texture_cache: HashMap<usize, wgpu::BindGroup>,
    pub default_texture_bind_group: wgpu::BindGroup,
    pub commands: Vec<RenderCommand>,
    pub scene_graph: HashMap<usize, SceneEntity>,
    /// Sprint 167: per-window dynamic light registry (max 4 active lights).
    pub lights: HashMap<usize, SceneLight>,
    // Egui State
    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
    pub ui_tree: Vec<crate::ast::Node>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RegistryVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3], // Sprint 85: Added - required by mesh3d.wgsl @location(1)
    pub tex_coords: [f32; 2],
}

pub struct CachedMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

// The types of resources we can manage
pub enum NativeHandle {
    Counter(StatefulCounter),
    Window(WindowProxy),
    File(File),
    Timestamp(std::time::Instant),
    GpuContext(GpuContext),
    VoxelWorld(SendVoxelWorld),
    Texture(TextureAsset),
}

pub struct RegistryEntry {
    pub handle: NativeHandle,
    pub ref_count: usize,
}

// Our dummy stateful Rust object
pub struct StatefulCounter {
    pub count: i64,
}

// GPU Context managed by the Registry
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

// SAFETY: wgpu GPU types are Send+Sync; our registry is single-threaded.
unsafe impl Send for GpuContext {}
unsafe impl Sync for GpuContext {}

pub struct TextureAsset {
    pub bind_group: Arc<wgpu::BindGroup>,
    pub width: u32,
    pub height: u32,
}
unsafe impl Send for TextureAsset {}
unsafe impl Sync for TextureAsset {}

// VoxelWorld — isometric software-rendered voxel scene
pub struct VoxelWorldState {
    pub width: usize,
    pub height: usize,
    pub voxels: Vec<[i32; 3]>,
}

pub struct SendVoxelWorld(pub VoxelWorldState);
unsafe impl Send for SendVoxelWorld {}
unsafe impl Sync for SendVoxelWorld {}

// ── Isometric software renderer ───────────────────────────────────────

/// Scanline polygon fill for convex polygons (used for isometric cube faces).
#[allow(dead_code)]
fn fill_poly(buffer: &mut [u32], width: usize, height: usize, pts: &[(i32, i32)], color: u32) {
    let min_y = pts.iter().map(|&(_, y)| y).min().unwrap_or(0).max(0) as usize;
    let raw_max = pts.iter().map(|&(_, y)| y).max().unwrap_or(0) as usize;
    let max_y = raw_max.min(height.saturating_sub(1));
    if min_y >= height {
        return;
    }
    let n = pts.len();
    for row in min_y..=max_y {
        let y = row as i32;
        let mut xs: Vec<i32> = Vec::new();
        for i in 0..n {
            let (x0, y0) = pts[i];
            let (x1, y1) = pts[(i + 1) % n];
            let (lo, hi, xa, xb) = if y0 < y1 {
                (y0, y1, x0, x1)
            } else {
                (y1, y0, x1, x0)
            };
            if lo <= y && y < hi && lo != hi {
                let t = (y - lo) as f32 / (hi - lo) as f32;
                xs.push((xa as f32 + t * (xb - xa) as f32) as i32);
            }
        }
        xs.sort_unstable();
        let mut i = 0;
        while i + 1 < xs.len() {
            let x0 = xs[i].max(0) as usize;
            let x1 = (xs[i + 1]).min(width as i32 - 1) as usize;
            if x0 <= x1 {
                for col in x0..=x1 {
                    buffer[row * width + col] = color;
                }
            }
            i += 2;
        }
    }
}

/// Isometric projection render — painters-sorted, 3-face-per-voxel.
#[allow(dead_code)]
fn iso_render(buffer: &mut [u32], width: usize, height: usize, voxels: &[[i32; 3]]) {
    buffer.iter_mut().for_each(|p| *p = 0x0d1b2a); // dark navy background
    let cx = (width as i32) / 2;
    let cy = (height as i32) * 5 / 8;
    let tw = 14i32; // half-width of one voxel tile
    let ts = 7i32; // half-height of top rhombus

    // Back-to-front sort: larger (vx + vz - vy*2) draws first
    let mut sorted: Vec<[i32; 3]> = voxels.to_vec();
    sorted.sort_by_key(|v| v[0] - v[1] * 2 + v[2]);

    for [vx, vy, vz] in sorted.iter() {
        let sx = cx + (vx - vz) * tw;
        let sy = cy + (vx + vz) * ts - vy * ts * 2;

        // Top face (rhombus)
        fill_poly(
            buffer,
            width,
            height,
            &[(sx, sy - ts), (sx + tw, sy), (sx, sy + ts), (sx - tw, sy)],
            0x5b9bd5,
        );
        // Left face (darker)
        fill_poly(
            buffer,
            width,
            height,
            &[
                (sx - tw, sy),
                (sx, sy + ts),
                (sx, sy + ts * 3),
                (sx - tw, sy + ts * 2),
            ],
            0x2e6ea8,
        );
        // Right face (darkest)
        fill_poly(
            buffer,
            width,
            height,
            &[
                (sx, sy + ts),
                (sx + tw, sy),
                (sx + tw, sy + ts * 2),
                (sx, sy + ts * 3),
            ],
            0x1a4a7c,
        );
    }
}

// Global thread-safe registry
// Instead of lazy_static we'll use a const Mutex with an Option since lazy_static might not be available
static COUNTER_REGISTRY: Mutex<Option<HashMap<usize, RegistryEntry>>> = Mutex::new(None);
static COUNTER_NEXT_ID: Mutex<usize> = Mutex::new(1);

fn with_registry<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<usize, RegistryEntry>) -> R,
{
    let mut option_guard = COUNTER_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    if option_guard.is_none() {
        *option_guard = Some(HashMap::new());
    }
    f(option_guard.as_mut().unwrap())
}

// ── Lifecycle FFI Implementations ─────────────────────────────────

pub fn registry_retain(handle_id: i64) {
    if handle_id < 0 {
        return;
    }
    let id = handle_id as usize;
    with_registry(|registry| {
        if let Some(entry) = registry.get_mut(&id) {
            entry.ref_count += 1;
        }
    });
}

pub fn registry_release(handle_id: i64) {
    if handle_id < 0 {
        return;
    }
    let id = handle_id as usize;
    let mut remove = false;
    with_registry(|registry| {
        if let Some(entry) = registry.get_mut(&id) {
            if entry.ref_count > 0 {
                entry.ref_count -= 1;
            }
            if entry.ref_count == 0 {
                remove = true;
            }
        }
        if remove {
            registry.remove(&id);
        }
    });
}

// FFI Implementations
pub fn registry_create_counter() -> i64 {
    let mut id_guard = COUNTER_NEXT_ID.lock().unwrap_or_else(|e| e.into_inner());
    let id = *id_guard;
    *id_guard += 1;

    let counter = StatefulCounter { count: 0 };
    with_registry(|registry| {
        registry.insert(
            id,
            RegistryEntry {
                handle: NativeHandle::Counter(counter),
                ref_count: 1,
            },
        );
    });

    id as i64
}

pub fn registry_increment(handle_id: i64) {
    if handle_id < 0 {
        return;
    }
    let id = handle_id as usize;
    with_registry(|registry| {
        if let Some(entry) = registry.get_mut(&id) {
            if let NativeHandle::Counter(counter) = &mut entry.handle {
                counter.count += 1;
            } else {
                eprintln!("[KnotenCore Registry] Error: Target handle is not a Counter.");
            }
        } else {
            eprintln!(
                "[KnotenCore Registry] Error: Counter handle {} not found.",
                handle_id
            );
        }
    });
}

pub fn registry_get_value(handle_id: i64) -> i64 {
    if handle_id < 0 {
        return 0;
    }
    let id = handle_id as usize;
    with_registry(|registry| {
        if let Some(entry) = registry.get(&id) {
            if let NativeHandle::Counter(counter) = &entry.handle {
                counter.count
            } else {
                -1
            }
        } else {
            eprintln!(
                "[KnotenCore Registry] Error: Counter handle {} not found.",
                handle_id
            );
            -1
        }
    })
}

pub fn registry_free(handle_id: i64) {
    if handle_id < 0 {
        return;
    }
    // Finding C-2: Do not unconditionally remove the handle, respect the refcount mechanism by releasing it
    registry_release(handle_id);
}

pub fn registry_dump() -> i64 {
    let mut count = 0;
    with_registry(|registry| {
        println!("[KnotenCore Registry] --- MEMORY DUMP ---");
        for (id, entry) in registry.iter() {
            let handle_type = match &entry.handle {
                NativeHandle::Counter(_) => "Counter",
                NativeHandle::Window(_) => "Window",
                NativeHandle::File(_) => "File",
                NativeHandle::Timestamp(_) => "Timestamp",
                NativeHandle::GpuContext(_) => "GpuContext",
                NativeHandle::VoxelWorld(SendVoxelWorld(s)) => {
                    println!("      voxels={}, {}x{}", s.voxels.len(), s.width, s.height);
                    "VoxelWorld"
                }
                NativeHandle::Texture(tex) => {
                    println!("      {}x{}", tex.width, tex.height);
                    "Texture"
                }
            };
            println!(
                "   -> Handle {} [Type: {}, RefCount: {}]",
                id, handle_type, entry.ref_count
            );
            count += 1;
        }
        println!("[KnotenCore Registry] Total Active: {}", count);
    });
    count
}

// ── Timestamp Orchestration ────────────────────────────────────────

pub fn registry_now() -> i64 {
    let mut id_guard = COUNTER_NEXT_ID.lock().unwrap_or_else(|e| e.into_inner());
    let id = *id_guard;
    *id_guard += 1;

    with_registry(|registry| {
        registry.insert(
            id,
            RegistryEntry {
                handle: NativeHandle::Timestamp(std::time::Instant::now()),
                ref_count: 1,
            },
        );
    });

    id as i64
}

pub fn registry_elapsed_ms(handle_id: i64) -> i64 {
    if handle_id < 0 {
        return 0;
    }
    let id = handle_id as usize;
    with_registry(|registry| {
        if let Some(entry) = registry.get(&id) {
            if let NativeHandle::Timestamp(t) = &entry.handle {
                t.elapsed().as_millis() as i64
            } else {
                -1
            }
        } else {
            -1
        }
    })
}

// ── Window Orchestration ─────────────────────────────────────────

pub fn registry_create_window(width: i64, height: i64, title: String) -> i64 {
    let mut id_guard = COUNTER_NEXT_ID.lock().unwrap_or_else(|e| e.into_inner());
    let id = *id_guard;
    *id_guard += 1;

    let w = width as u32;
    let h = height as u32;

    let input = Arc::new(Mutex::new(InputState {
        keys: HashSet::new(),
        mouse_dx: 0.0,
        mouse_dy: 0.0,
        mouse_x: 0.0,
        mouse_y: 0.0,
        mouse_left_down: false,
        mouse_clicked: false,
        view_proj: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        window_width: w as f32,
        window_height: h as f32,
        last_char: 0,
    }));

    send_render_command(RenderCommand::CreateWindow {
        id,
        title,
        width: w,
        height: h,
        input: input.clone(),
    });

    with_registry(|registry| {
        registry.insert(
            id,
            RegistryEntry {
                handle: NativeHandle::Window(WindowProxy { id, input }),
                ref_count: 1,
            },
        );
    });

    id as i64
}

pub fn registry_window_update(handle_id: i64) -> bool {
    if handle_id < 0 {
        return false;
    }
    let id = handle_id as usize;
    send_render_command(RenderCommand::UpdateWindow(id));

    // We assume the window is open unless we receive a message back or have a way to check.
    // For now, we return true. The main loop will handle window closure.
    true
}

pub fn registry_window_close(handle_id: i64) {
    if handle_id < 0 {
        return;
    }
    let id = handle_id as usize;
    send_render_command(RenderCommand::CloseWindow(id));
    registry_free(handle_id);
}

// ── File IO Orchestration ─────────────────────────────────────────

pub fn registry_file_create(path: String) -> i64 {
    let mut id_guard = COUNTER_NEXT_ID.lock().unwrap_or_else(|e| e.into_inner());
    let id = *id_guard;
    *id_guard += 1;

    let safe_path = match crate::executor::ExecutionEngine::validate_fs_path_write(&path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "[KnotenCore FileIO] Security error creating file '{}': {}",
                path, e
            );
            return -1;
        }
    };

    match File::create(&safe_path) {
        Ok(file) => {
            with_registry(|registry| {
                registry.insert(
                    id,
                    RegistryEntry {
                        handle: NativeHandle::File(file),
                        ref_count: 1,
                    },
                );
            });
            id as i64
        }
        Err(e) => {
            eprintln!("[KnotenCore FileIO] Error creating file '{}': {}", path, e);
            -1
        }
    }
}

pub fn registry_file_write(handle_id: i64, content: String) {
    if handle_id < 0 {
        return;
    }
    let id = handle_id as usize;
    with_registry(|registry| {
        if let Some(entry) = registry.get_mut(&id) {
            if let NativeHandle::File(file) = &mut entry.handle {
                if let Err(e) = file.write_all(content.as_bytes()) {
                    eprintln!(
                        "[KnotenCore FileIO] Failed to write to file handle {}: {}",
                        handle_id, e
                    );
                }
            } else {
                eprintln!("[KnotenCore FileIO] Handle {} is not a File.", handle_id);
            }
        } else {
            eprintln!("[KnotenCore FileIO] Handle {} not found.", handle_id);
        }
    });
}

// ── GPU Orchestration ────────────────────────────────────────────────

pub fn registry_gpu_init() -> i64 {
    let mut id_guard = COUNTER_NEXT_ID.lock().unwrap_or_else(|e| e.into_inner());
    let id = *id_guard;
    *id_guard += 1;

    // This is synchronous and can be slow, but it's called once.
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("Failed to find WGPU adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("KnotenCore GPU Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        },
        None,
    ))
    .expect("Failed to create WGPU device");

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    with_registry(|registry| {
        registry.insert(
            id,
            RegistryEntry {
                handle: NativeHandle::GpuContext(GpuContext {
                    instance,
                    adapter,
                    device,
                    queue,
                }),
                ref_count: 1,
            },
        );
    });

    id as i64
}

pub fn registry_fill_color(window_handle: i64, _r: i64, _g: i64, _b: i64) {
    if window_handle < 0 {}
    // Note: We could send a Command for this too.
}

// ── Voxel World Orchestration ─────────────────────────────────────────

pub fn registry_voxel_world_create(_width: i64, _height: i64, _title: String) -> i64 {
    eprintln!("[KnotenCore Voxel] Legacy Voxel module disabled in Sprint 51.");
    -1
}

pub fn registry_voxel_add_block(_world_handle: i64, _x: i64, _y: i64, _z: i64) {}

/// Renders one frame of the voxel scene. Returns true while the window is open.
pub fn registry_voxel_render_frame(_world_handle: i64) -> bool {
    false
}

pub struct RegistryModule;

impl crate::natives::NativeModule for RegistryModule {
    fn handle(
        &self,
        func_name: &str,
        args: &[crate::executor::RelType],
        permissions: &crate::executor::AgentPermissions,
    ) -> Option<crate::executor::ExecResult> {
        use crate::natives::bridge::BridgeModule;
        crate::natives::bridge::CoreBridge.handle("registry", func_name, args, permissions)
    }
}

// ── Texture Orchestration ─────────────────────────────────────────

pub fn registry_texture_load(path: String) -> i64 {
    let safe_path = match crate::executor::ExecutionEngine::validate_fs_path(&path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "[KnotenCore Texture] Security error loading '{}': {}",
                path, e
            );
            return -1;
        }
    };
    let img = match image::open(&safe_path) {
        Ok(img) => img.to_rgba8(),
        Err(e) => {
            eprintln!("[KnotenCore Texture] Failed to load '{}': {}", path, e);
            return -1;
        }
    };
    let dimensions = img.dimensions();

    let (device, queue) = with_registry(|registry| {
        for entry in registry.values() {
            if let NativeHandle::GpuContext(ctx) = &entry.handle {
                return Some((ctx.device.clone(), ctx.queue.clone()));
            }
        }
        None
    })
    .expect("Cannot load texture without an active WGPU context. Call registry_gpu_init or create a window first.");

    let texture_size = wgpu::Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };

    let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some(&path),
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &diffuse_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &img,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * dimensions.0),
            rows_per_image: Some(dimensions.1),
        },
        texture_size,
    );

    let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
        label: Some("texture_bind_group_layout"),
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
            },
        ],
        label: Some("diffuse_bind_group"),
    });

    let mut id_guard = COUNTER_NEXT_ID.lock().unwrap_or_else(|e| e.into_inner());
    let id = *id_guard;
    *id_guard += 1;

    with_registry(|registry| {
        registry.insert(
            id,
            RegistryEntry {
                handle: NativeHandle::Texture(TextureAsset {
                    bind_group: Arc::new(bind_group),
                    width: dimensions.0,
                    height: dimensions.1,
                }),
                ref_count: 1,
            },
        );
    });

    id as i64
}

static NEXT_ENTITY_ID: std::sync::Mutex<usize> = std::sync::Mutex::new(1);
static NEXT_LIGHT_ID: std::sync::Mutex<usize> = std::sync::Mutex::new(1);

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
    let mut guard = SENT_MESHES.lock().unwrap();
    let sent = guard.get_or_insert_with(HashSet::new);
    if !sent.contains(&mesh_name) {
        let (vertices, indices) = generate_cube();
        send_render_command(RenderCommand::AddMesh {
            name: mesh_name.clone(),
            vertices,
            indices,
        });
        sent.insert(mesh_name.clone());
    }
    drop(guard);

    let mut id_guard = NEXT_ENTITY_ID.lock().unwrap();
    let entity_id = *id_guard;
    *id_guard += 1;

    let t = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
    let s = glam::Mat4::from_scale(glam::Vec3::new(w, h, d));
    let transform = t * s;

    let base_aabb = crate::math::AABB::new([-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]);
    let world_aabb = base_aabb.transform(&transform);
    let mut phys_guard = PHYSICS_WORLD.lock().unwrap();
    let phys_map = phys_guard.get_or_insert_with(HashMap::new);
    phys_map.insert(
        entity_id,
        EntityPhysics {
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
        transform,
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

    // Retained Physics: preserve old scale and update AABB
    let mut phys_guard = PHYSICS_WORLD.lock().unwrap();
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
        transform: final_transform,
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

    // Remove from physics world — track whether it actually existed
    let mut phys_guard = PHYSICS_WORLD.lock().unwrap();
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

    // Send render command to remove from scene graph (idempotent)
    send_render_command(RenderCommand::RemoveEntity {
        window_id: window_handle as usize,
        entity_id: eid,
    });
}

/// Sprint 167: Spawn a dynamic point light into the scene.
/// Returns a unique light ID that can be used to update its position later.
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
    let mut id_guard = NEXT_LIGHT_ID.lock().unwrap();
    let light_id = *id_guard;
    *id_guard += 1;

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

/// Sprint 167: Update the position of an existing point light.
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

    let mut guard = SENT_MESHES.lock().unwrap();
    let sent = guard.get_or_insert_with(HashSet::new);
    if !sent.contains(&mesh_name) {
        let (vertices, indices) = generate_uv_sphere(rings, sectors);
        send_render_command(RenderCommand::AddMesh {
            name: mesh_name.clone(),
            vertices,
            indices,
        });
        sent.insert(mesh_name.clone());
    }
    drop(guard);

    let mut id_guard = NEXT_ENTITY_ID.lock().unwrap();
    let entity_id = *id_guard;
    *id_guard += 1;

    let t = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
    let s = glam::Mat4::from_scale(glam::Vec3::splat(radius));
    let transform = t * s;

    let base_aabb = crate::math::AABB::new([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);
    let world_aabb = base_aabb.transform(&transform);
    let mut phys_guard = PHYSICS_WORLD.lock().unwrap();
    let phys_map = phys_guard.get_or_insert_with(HashMap::new);
    phys_map.insert(
        entity_id,
        EntityPhysics {
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
        transform,
    });
    entity_id as i64
}

fn generate_uv_sphere(rings: u32, sectors: u32) -> (Vec<RegistryVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for r in 0..=rings {
        let phi = std::f32::consts::PI * (r as f32 / rings as f32);
        for s in 0..=sectors {
            let theta = 2.0 * std::f32::consts::PI * (s as f32 / sectors as f32);
            let nx = phi.sin() * theta.cos();
            let ny = phi.cos();
            let nz = phi.sin() * theta.sin();
            let u = s as f32 / sectors as f32;
            let v = r as f32 / rings as f32;
            // For a unit sphere, position == outward normal
            vertices.push(RegistryVertex {
                position: [nx, ny, nz],
                normal: [nx, ny, nz],
                tex_coords: [u, v],
            });
        }
    }

    for r in 0..rings {
        for s in 0..sectors {
            let first = r * (sectors + 1) + s;
            let second = first + sectors + 1;
            indices.push(first);
            indices.push(second);
            indices.push(first + 1);
            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }
    (vertices, indices)
}

// Cube generator is handled by registry_spawn_cube directly since we need the mesh generated there

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

    let mut guard = SENT_MESHES.lock().unwrap();
    let sent = guard.get_or_insert_with(HashSet::new);
    if !sent.contains(&mesh_name) {
        let (vertices, indices) = generate_cylinder(segments);
        send_render_command(RenderCommand::AddMesh {
            name: mesh_name.clone(),
            vertices,
            indices,
        });
        sent.insert(mesh_name.clone());
    }
    drop(guard);

    let mut id_guard = NEXT_ENTITY_ID.lock().unwrap();
    let entity_id = *id_guard;
    *id_guard += 1;

    let t = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
    let s = glam::Mat4::from_scale(glam::Vec3::new(radius, height, radius));
    let transform = t * s;

    let base_aabb = crate::math::AABB::new([-1.0, -0.5, -1.0], [1.0, 0.5, 1.0]);
    let world_aabb = base_aabb.transform(&transform);
    let mut phys_guard = PHYSICS_WORLD.lock().unwrap();
    let phys_map = phys_guard.get_or_insert_with(HashMap::new);
    phys_map.insert(
        entity_id,
        EntityPhysics {
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
        transform,
    });
    entity_id as i64
}

fn generate_cylinder(segments: u32) -> (Vec<RegistryVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Top center (normal points up)
    vertices.push(RegistryVertex {
        position: [0.0, 0.5, 0.0],
        normal: [0.0, 1.0, 0.0],
        tex_coords: [0.5, 0.5],
    });
    // Bottom center (normal points down)
    vertices.push(RegistryVertex {
        position: [0.0, -0.5, 0.0],
        normal: [0.0, -1.0, 0.0],
        tex_coords: [0.5, 0.5],
    });

    let base_idx_top: u32 = 0;
    let base_idx_bottom: u32 = 1;

    for i in 0..=segments {
        let theta = 2.0 * std::f32::consts::PI * (i as f32 / segments as f32);
        let x = theta.cos();
        let z = theta.sin();
        let u = i as f32 / segments as f32;
        // Side normals point outward horizontally
        let nx = x;
        let nz = z;
        // Top cap vertex
        vertices.push(RegistryVertex {
            position: [x, 0.5, z],
            normal: [nx, 0.0, nz],
            tex_coords: [u, 0.0],
        });
        // Bottom cap vertex
        vertices.push(RegistryVertex {
            position: [x, -0.5, z],
            normal: [nx, 0.0, nz],
            tex_coords: [u, 1.0],
        });
    }

    for i in 0..segments {
        let top0 = 2 + i * 2;
        let bot0 = top0 + 1;
        let top1 = top0 + 2;
        let bot1 = top1 + 1;

        // Side faces
        indices.push(top0);
        indices.push(bot0);
        indices.push(top1);
        indices.push(bot0);
        indices.push(bot1);
        indices.push(top1);

        // Top cap
        indices.push(base_idx_top);
        indices.push(top1);
        indices.push(top0);

        // Bottom cap
        indices.push(base_idx_bottom);
        indices.push(bot0);
        indices.push(bot1);
    }
    (vertices, indices)
}

/// Sprint 85: Generate a unit cube with per-face flat normals.
fn generate_cube() -> (Vec<RegistryVertex>, Vec<u32>) {
    // 6 faces × 4 vertices = 24 vertices; 6 faces × 2 triangles × 3 = 36 indices
    #[allow(clippy::type_complexity)]
    let faces: [([f32; 3], [[f32; 3]; 4], [[f32; 2]; 4]); 6] = [
        // +Y top
        (
            [0.0, 1.0, 0.0],
            [
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ),
        // -Y bottom
        (
            [0.0, -1.0, 0.0],
            [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
            ],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ),
        // +Z front
        (
            [0.0, 0.0, 1.0],
            [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
        // -Z back
        (
            [0.0, 0.0, -1.0],
            [
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
        // +X right
        (
            [1.0, 0.0, 0.0],
            [
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
        // -X left
        (
            [-1.0, 0.0, 0.0],
            [
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
    ];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    for (face_idx, (normal, positions, uvs)) in faces.iter().enumerate() {
        let base = (face_idx * 4) as u32;
        for (pos, uv) in positions.iter().zip(uvs.iter()) {
            vertices.push(RegistryVertex {
                position: *pos,
                normal: *normal,
                tex_coords: *uv,
            });
        }
        // Two CCW triangles per face
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (vertices, indices)
}

/// Sprint 86: Compute a perspective view-proj matrix and send it to all windows.
/// Called from scripts as: registry_set_camera(fov_deg, cam_x, cam_y, cam_z)
pub fn registry_set_camera(fov_degrees: f32, cam_x: f32, cam_y: f32, cam_z: f32) {
    registry_set_camera_for_window(0, fov_degrees, cam_x, cam_y, cam_z);
}

/// Sprint 86: Send a camera to a specific window (window_id = handle_id).
/// Called from scripts as: registry_set_camera_for_window(win, fov_deg, cam_x, cam_y, cam_z)
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
    // Assume a reasonable aspect until the window reports its size via resize events.
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

pub fn registry_get_mouse_delta_x() -> f32 {
    let mut acc = 0.0;
    with_registry(|registry| {
        for entry in registry.values() {
            if let NativeHandle::Window(proxy) = &entry.handle {
                let input = proxy.input.lock().unwrap_or_else(|e| e.into_inner());
                acc += input.mouse_dx;
            }
        }
    });
    acc
}

pub fn registry_get_mouse_delta_y() -> f32 {
    let mut acc = 0.0;
    with_registry(|registry| {
        for entry in registry.values() {
            if let NativeHandle::Window(proxy) = &entry.handle {
                let input = proxy.input.lock().unwrap_or_else(|e| e.into_inner());
                acc += input.mouse_dy;
            }
        }
    });
    acc
}

pub fn registry_is_mouse_down() -> bool {
    let mut pressed = false;
    with_registry(|registry| {
        for entry in registry.values() {
            if let NativeHandle::Window(proxy) = &entry.handle {
                let input = proxy.input.lock().unwrap_or_else(|e| e.into_inner());
                if input.mouse_left_down {
                    pressed = true;
                }
            }
        }
    });
    pressed
}

pub fn registry_get_mouse_ray(window_handle: i64) -> Vec<crate::executor::RelType> {
    use glam::{Mat4, Vec3};
    if window_handle < 0 {
        return vec![];
    }
    let id = window_handle as usize;
    let mut ray_origin = Vec3::ZERO;
    let mut ray_dir = Vec3::Z; // Default into screen

    with_registry(|registry| {
        if let Some(entry) = registry.get(&id)
            && let NativeHandle::Window(proxy) = &entry.handle
        {
            let input = proxy.input.lock().unwrap_or_else(|e| e.into_inner());
            let mx = input.mouse_x;
            let my = input.mouse_y;
            let w = input.window_width.max(1.0);
            let h = input.window_height.max(1.0);
            let vp = Mat4::from_cols_array_2d(&input.view_proj);

            // Screen to NDC (Normalized Device Coordinates [-1, 1])
            // Y goes down in window, but up in NDC
            let ndc_x = (2.0 * mx) / w - 1.0;
            let ndc_y = 1.0 - (2.0 * my) / h;

            let inv_vp = vp.inverse();

            // Project points at near and far plane
            let near_pt = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
            let far_pt = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));

            ray_origin = near_pt;
            ray_dir = (far_pt - near_pt).normalize();
        }
    });

    vec![
        crate::executor::RelType::Float(ray_origin.x as f64),
        crate::executor::RelType::Float(ray_origin.y as f64),
        crate::executor::RelType::Float(ray_origin.z as f64),
        crate::executor::RelType::Float(ray_dir.x as f64),
        crate::executor::RelType::Float(ray_dir.y as f64),
        crate::executor::RelType::Float(ray_dir.z as f64),
    ]
}

pub fn registry_get_last_char() -> i64 {
    let mut last = 0;
    with_registry(|registry| {
        for entry in registry.values() {
            if let NativeHandle::Window(proxy) = &entry.handle {
                let input = proxy.input.lock().unwrap_or_else(|e| e.into_inner());
                if input.last_char != 0 {
                    last = input.last_char as i64;
                }
            }
        }
    });
    last
}

pub fn registry_read_file(path: String) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|_| "".to_string())
}

pub fn registry_write_file(path: String, content: String) -> bool {
    std::fs::write(&path, content).is_ok()
}

pub fn registry_get_ultimate_answer() -> i64 {
    42
}

/// Sprint 170: Debug FFI function that intentionally panics to test the
/// panic-safety layer in the VM bridge. Must be caught by `catch_unwind`
/// in machine.rs — the application must NOT crash.
pub fn registry_force_panic() {
    panic!("Simulated core dump from FFI!");
}

// ── Assets & Textures (Sprint 165) ──────────────────────────────────────

pub fn registry_load_texture(path: &str) -> i64 {
    let img_result = image::open(path);
    if let Ok(img) = img_result {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let raw_data = rgba.into_raw();

        let mut id_guard = TEXTURE_ID_COUNTER.lock().unwrap();
        let id = *id_guard;
        *id_guard += 1;

        send_render_command(RenderCommand::LoadTexture {
            id,
            width,
            height,
            rgba: raw_data,
        });

        return id as i64;
    }
    // Return 0 (default texture) on error
    0
}

// ── Physics & Raycasting (Sprint 164) ───────────────────────────────────

pub fn registry_check_collision(id1: i64, id2: i64) -> bool {
    let guard = PHYSICS_WORLD.lock().unwrap();
    if let Some(map) = guard.as_ref()
        && let (Some(e1), Some(e2)) = (map.get(&(id1 as usize)), map.get(&(id2 as usize)))
    {
        return e1.world_aabb.intersects(&e2.world_aabb);
    }
    false
}

pub fn registry_get_clicked_entity(window_handle: i64) -> i64 {
    if window_handle < 0 {
        return -1;
    }
    let input = with_registry(|reg| {
        if let Some(entry) = reg.get(&(window_handle as usize))
            && let NativeHandle::Window(proxy) = &entry.handle
        {
            return Some(proxy.input.clone());
        }
        None
    });

    if let Some(input_arc) = input {
        let mut state = input_arc.lock().unwrap();
        if state.mouse_clicked {
            state.mouse_clicked = false; // Consume click

            // Unproject mouse coordinates to ray
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

            // Raycast against physics world
            let guard = PHYSICS_WORLD.lock().unwrap();
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
