use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::AtomicI64;

use crossbeam_channel::{Receiver, Sender, bounded};

use std::collections::HashSet;
use winit::keyboard::KeyCode;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// Sprint 184: Re-exports from extracted modules for backward compatibility
pub use super::geometry::{
    CachedMesh, RegistryVertex, generate_cube, generate_cylinder, generate_uv_sphere,
};
pub use super::physics::{
    EntityPhysics, PHYSICS_WORLD, registry_check_collision, registry_get_clicked_entity,
};
pub use super::scene::{
    RegistryWindowState, SceneEntity, SceneLight, registry_destroy_entity, registry_set_camera,
    registry_set_camera_for_window, registry_spawn_cube, registry_spawn_cylinder,
    registry_spawn_light, registry_spawn_sphere, registry_update_entity_transform,
    registry_update_light_position,
};

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
        nodes: Vec<knoten_core_types::ast::Node>,
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
    // Sprint 210: Async texture load failure fallback
    LoadTextureFailed {
        id: usize,
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
        bindings: Option<Vec<Vec<crate::executor::RelType>>>,
    },
    // Sprint 242: Multi-pass compute chain — sequential shader passes in one encoder
    ComputeChain {
        shader_id: usize,
        steps: Vec<ComputeChainStep>,
    },
    // Sprint 204: Read back compute shader results from GPU to VM
    ReadComputeResult {
        shader_id: usize,
    },
    RemoveEntity {
        window_id: usize,
        entity_id: usize,
    },
    ExitEventLoop,
    // Sprint 257: GPU-accelerated UI hit-testing
    UiHitTest {
        panel_aabbs: Vec<crate::executor::RelType>,
        mouse_x: f32,
        mouse_y: f32,
    },
}

#[derive(Clone)]
pub struct ComputeChainStep {
    pub shader_id: usize,
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub inputs: Vec<crate::executor::RelType>,
    pub bindings: Option<Vec<Vec<crate::executor::RelType>>>,
}

pub fn exit_event_loop() {
    send_render_command(RenderCommand::ExitEventLoop);
}

static RENDER_TX: Mutex<Option<winit::event_loop::EventLoopProxy<RenderCommand>>> =
    Mutex::new(None);
pub static AUDIO_STATE: Mutex<Option<crate::audio::AudioManager>> = Mutex::new(None);

pub fn init_audio_state() {
    let mut guard = AUDIO_STATE.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none()
        && let Ok(manager) = crate::audio::AudioManager::new()
    {
        *guard = Some(manager);
    }
}

pub fn registry_play_boot_tone() {
    init_audio_state();
    if let Ok(mut guard) = AUDIO_STATE.lock()
        && let Some(ref mut mgr) = *guard
    {
        mgr.play_tone(
            0,
            440.0,
            150,
            0.3,
            knoten_core_types::ast::Waveform::Sine,
            5,
            20,
            0.7,
            100,
        );
    }
}

pub fn registry_play_tone_panned(
    channel: i64,
    freq: f64,
    duration_ms: i64,
    waveform_idx: i64,
    pan: f64,
) -> Result<(), String> {
    if !(-1.0..=1.0).contains(&pan) {
        return Err(format!("Pan value {} out of range [-1.0, 1.0]", pan));
    }
    init_audio_state();
    let waveform = match waveform_idx {
        0 => knoten_core_types::ast::Waveform::Sine,
        1 => knoten_core_types::ast::Waveform::Sawtooth,
        2 => knoten_core_types::ast::Waveform::Square,
        3 => knoten_core_types::ast::Waveform::Triangle,
        _ => knoten_core_types::ast::Waveform::Sine,
    };
    if let Ok(mut guard) = AUDIO_STATE.lock()
        && let Some(ref mut mgr) = *guard
    {
        mgr.play_tone_panned(
            channel as usize,
            freq as f32,
            duration_ms as u64,
            0.3,
            waveform,
            5,
            20,
            0.7,
            100,
            pan as f32,
        );
    }
    Ok(())
}

pub fn registry_play_sound(path: &str) -> Result<(), String> {
    init_audio_state();
    if let Ok(mut guard) = AUDIO_STATE.lock()
        && let Some(ref mut mgr) = *guard
    {
        mgr.play_sound(path)?;
        return Ok(());
    }
    Err("Audio engine not initialized".into())
}

pub fn registry_loop_music(path: &str) -> Result<(), String> {
    init_audio_state();
    if let Ok(mut guard) = AUDIO_STATE.lock()
        && let Some(ref mut mgr) = *guard
    {
        mgr.loop_music(path)?;
        return Ok(());
    }
    Err("Audio engine not initialized".into())
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
pub fn send_ui_nodes_to(window_id: usize, nodes: Vec<knoten_core_types::ast::Node>) {
    send_render_command(RenderCommand::UpdateUI { window_id, nodes });
}

/// Legacy helper: broadcasts to window 1 (single-window scripts).
pub fn send_ui_nodes(nodes: Vec<knoten_core_types::ast::Node>) {
    send_ui_nodes_to(1, nodes);
}

/// Sprint 162: Set the retained UI tree for a window.
/// Accepts a window handle (i64) and a vector of AST nodes.
/// The render thread will autonomously draw this tree at 60 FPS.
pub fn registry_ui_set(window_handle: i64, nodes: Vec<knoten_core_types::ast::Node>) {
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

pub static TEXTURE_ID_COUNTER: AtomicUsize = AtomicUsize::new(1); // 0 is reserved for default

// Sprint 213/215: Per-shader lock-free async compute channels
type ComputeChannel = (Sender<Vec<f32>>, Receiver<Vec<f32>>);
static COMPUTE_CHANNELS: OnceLock<Mutex<HashMap<usize, ComputeChannel>>> = OnceLock::new();

fn ensure_channel_for(shader_id: usize) {
    let channels = COMPUTE_CHANNELS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = channels.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .entry(shader_id)
        .or_insert_with(|| bounded::<Vec<f32>>(1));
}

pub fn compute_sender_for(shader_id: usize) -> Sender<Vec<f32>> {
    ensure_channel_for(shader_id);
    let channels = COMPUTE_CHANNELS.get().unwrap();
    let guard = channels.lock().unwrap_or_else(|e| e.into_inner());
    guard.get(&shader_id).unwrap().0.clone()
}

// Sprint 233: Native SIMD matrix storage for transpose/transform operations
static MATRIX_REGISTRY: OnceLock<Mutex<HashMap<i64, glam::Mat4>>> = OnceLock::new();
static MATRIX_ID_COUNTER: AtomicI64 = AtomicI64::new(1);

pub fn registry_store_matrix(mat: glam::Mat4) -> i64 {
    let id = MATRIX_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let registry = MATRIX_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    registry
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(id, mat);
    id
}

pub fn registry_get_matrix(handle: i64) -> Option<glam::Mat4> {
    MATRIX_REGISTRY.get().and_then(|r| {
        r.lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&handle)
            .copied()
    })
}

pub fn registry_transpose_matrix(handle: i64) -> Option<i64> {
    let mat = registry_get_matrix(handle)?;
    let transposed = mat.transpose();
    Some(registry_store_matrix(transposed))
}

// Sprint 243: Profiler timing markers for GPGPU chain execution
static PROFILER_MARKERS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn registry_push_timing_marker(marker: String) {
    if let Ok(mut guard) = PROFILER_MARKERS.lock() {
        guard.push(marker);
    }
}

pub fn registry_drain_timing_markers() -> Vec<String> {
    let mut guard = PROFILER_MARKERS.lock().unwrap_or_else(|e| e.into_inner());
    std::mem::take(&mut *guard)
}

// Sprint 255: Self-healing failure tracker
static FAILURE_TRACKER: OnceLock<Mutex<HashMap<String, usize>>> = OnceLock::new();

fn get_failure_tracker() -> &'static Mutex<HashMap<String, usize>> {
    FAILURE_TRACKER.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn registry_track_failure(module: &str) {
    let mut guard = get_failure_tracker()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let count = guard.entry(module.to_string()).or_insert(0);
    *count += 1;
    if *count >= 5 {
        eprintln!(
            "[SelfHealing] Module '{}' reached {} failures — triggering reset",
            module, *count
        );
        registry_reset_module(module);
        *count = 0;
    }
}

pub fn registry_reset_module(module_name: &str) {
    match module_name {
        "audio" | "registry" => {
            init_audio_state();
            eprintln!(
                "[SelfHealing] Audio state reinitialized for module '{}'",
                module_name
            );
        }
        _ => {
            eprintln!(
                "[SelfHealing] Reset requested for unknown module '{}' — no action",
                module_name
            );
        }
    }
}

pub fn registry_get_failure_count(module: &str) -> usize {
    get_failure_tracker()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(module)
        .copied()
        .unwrap_or(0)
}

pub fn registry_drain_failure_tracker() {
    get_failure_tracker()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

// Sprint 258: Agent latency monitoring for LLM feedback loop
static LATENCY_MONITOR: OnceLock<Mutex<HashMap<String, Vec<u128>>>> = OnceLock::new();
static LATENCY_TIMERS: OnceLock<Mutex<HashMap<String, std::time::Instant>>> = OnceLock::new();

fn get_latency_monitor() -> &'static Mutex<HashMap<String, Vec<u128>>> {
    LATENCY_MONITOR.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_latency_timers() -> &'static Mutex<HashMap<String, std::time::Instant>> {
    LATENCY_TIMERS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn registry_start_latency_timer(command_id: &str) {
    let mut timers = get_latency_timers()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    timers.insert(command_id.to_string(), std::time::Instant::now());
}

pub fn registry_stop_latency_timer(command_id: &str) -> u128 {
    let mut timers = get_latency_timers()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let elapsed = if let Some(start) = timers.remove(command_id) {
        start.elapsed().as_micros()
    } else {
        0
    };
    let mut monitor = get_latency_monitor()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    monitor
        .entry(command_id.to_string())
        .or_default()
        .push(elapsed);
    elapsed
}

pub fn registry_get_avg_latency(command_id: &str) -> f64 {
    let monitor = get_latency_monitor()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(measurements) = monitor.get(command_id) {
        if measurements.is_empty() {
            return 0.0;
        }
        let sum: u128 = measurements.iter().sum();
        (sum as f64) / (measurements.len() as f64)
    } else {
        0.0
    }
}

pub fn registry_drain_latency_data() {
    get_latency_monitor()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
    get_latency_timers()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

// Sprint 247: VM inspection FFI — expose runtime state
pub fn registry_vm_get_ip() -> i64 {
    crate::vm::machine::get_vm_inspection_snapshot()
        .map(|(ip, _)| ip as i64)
        .unwrap_or(-1)
}

pub fn registry_vm_get_stack_depth() -> i64 {
    crate::vm::machine::get_vm_inspection_snapshot()
        .map(|(_, depth)| depth as i64)
        .unwrap_or(-1)
}

pub fn registry_ui_hit_test(
    panel_aabbs: Vec<crate::executor::RelType>,
    mouse_x: f64,
    mouse_y: f64,
) {
    send_render_command(RenderCommand::UiHitTest {
        panel_aabbs,
        mouse_x: mouse_x as f32,
        mouse_y: mouse_y as f32,
    });
}

// Sprint 261: Lock-free mailbox for cross-isolate communication
static MAILBOX_REGISTRY: OnceLock<Mutex<HashMap<i64, Sender<crate::executor::RelType>>>> =
    OnceLock::new();

pub fn registry_register_mailbox(isolate_id: i64) -> Receiver<crate::executor::RelType> {
    let (tx, rx) = bounded::<crate::executor::RelType>(16);
    let registry = MAILBOX_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    registry
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(isolate_id, tx);
    rx
}

pub fn registry_send_message(target_id: i64, message: crate::executor::RelType) -> bool {
    let registry = MAILBOX_REGISTRY.get();
    if let Some(reg) = registry
        && let Ok(guard) = reg.lock()
        && let Some(sender) = guard.get(&target_id)
    {
        return sender.try_send(message).is_ok();
    }
    false
}

pub fn registry_receive_message(_isolate_id: i64) -> Option<crate::executor::RelType> {
    MAILBOX_REGISTRY.get().and_then(|reg| {
        reg.lock().ok().and_then(|_| {
            // Channels are consumed from the receiver side, which is held by the isolate.
            // This FFI is for testing/external access.
            None
        })
    })
}

pub fn registry_drain_mailbox_registry() {
    if let Some(reg) = MAILBOX_REGISTRY.get() {
        reg.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

unsafe impl Send for WindowProxy {}
unsafe impl Sync for WindowProxy {}

// The types of resources we can manage
pub enum NativeHandle {
    Counter(StatefulCounter),
    Window(WindowProxy),
    File(File),
    Timestamp(std::time::Instant),
    GpuContext(GpuContext),
    Texture(TextureAsset),
}

pub struct RegistryEntry {
    pub handle: NativeHandle,
    pub ref_count: usize,
}

// Our dummy stateful Rust object
pub struct StatefulCounter {
    pub count: AtomicI64,
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

// ── Isometric software renderer removed in Sprint 176 — superseded by WGPU 3D pipeline. ──
// ── VoxelWorldState, SendVoxelWorld, NativeHandle::VoxelWorld removed in Sprint 184. ──

// Sprint 267: Lock-free native resource handles
static COUNTER_REGISTRY: OnceLock<Mutex<HashMap<usize, RegistryEntry>>> = OnceLock::new();
static COUNTER_NEXT_ID: AtomicUsize = AtomicUsize::new(1);

fn get_counter_registry() -> &'static Mutex<HashMap<usize, RegistryEntry>> {
    COUNTER_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn with_registry<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<usize, RegistryEntry>) -> R,
{
    let mut guard = get_counter_registry()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
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
    let id = COUNTER_NEXT_ID.fetch_add(1, Ordering::Relaxed);

    let counter = StatefulCounter {
        count: AtomicI64::new(0),
    };
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
        if let Some(entry) = registry.get(&id) {
            if let NativeHandle::Counter(counter) = &entry.handle {
                counter.count.fetch_add(1, Ordering::Relaxed);
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
                counter.count.load(Ordering::Relaxed)
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
    let id = COUNTER_NEXT_ID.fetch_add(1, Ordering::Relaxed);

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
    let id = COUNTER_NEXT_ID.fetch_add(1, Ordering::Relaxed);

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
    let id = COUNTER_NEXT_ID.fetch_add(1, Ordering::Relaxed);

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
    let id = COUNTER_NEXT_ID.fetch_add(1, Ordering::Relaxed);

    // This is synchronous and can be slow, but it's called once.
    let instance = wgpu::Instance::default();
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) {
        Some(a) => a,
        None => {
            eprintln!("[KnotenCore GPU] Failed to find WGPU adapter");
            return -1;
        }
    };

    let (device, queue) = match pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("KnotenCore GPU Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        },
        None,
    )) {
        Ok(dq) => dq,
        Err(e) => {
            eprintln!("[KnotenCore GPU] Failed to create WGPU device: {}", e);
            return -1;
        }
    };

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

    let (device, queue) = match with_registry(|registry| {
        for entry in registry.values() {
            if let NativeHandle::GpuContext(ctx) = &entry.handle {
                return Some((ctx.device.clone(), ctx.queue.clone()));
            }
        }
        None
    }) {
        Some(dq) => dq,
        None => {
            eprintln!(
                "[KnotenCore Texture] Cannot load texture '{}' — no active WGPU context. Call registry_gpu_init or create a window first.",
                path
            );
            return -1;
        }
    };

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

    let id = COUNTER_NEXT_ID.fetch_add(1, Ordering::Relaxed);

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

pub fn registry_load_compute_shader(source: String) -> i64 {
    let id = rand::random::<i64>().abs();
    send_render_command(RenderCommand::LoadComputeShader {
        id: id as usize,
        source,
    });
    id
}

pub fn registry_dispatch_compute(
    shader_id: i64,
    x: u32,
    y: u32,
    z: u32,
    inputs: Vec<crate::executor::RelType>,
) {
    send_render_command(RenderCommand::DispatchCompute {
        shader_id: shader_id as usize,
        x,
        y,
        z,
        inputs,
        bindings: None,
    });
}

// Sprint 215: Fixed readback — per-shader channel, no drain, spin-poll for async results
// #ANCHOR: GPGPU_ASYNC_CHANNEL — Lock-free crossbeam-channel try_recv endpoint for VM compute readback.
pub fn registry_compute_readback(shader_id: i64) -> Vec<crate::executor::RelType> {
    let sid = shader_id as usize;
    ensure_channel_for(sid);

    // Fire readback command to the render thread (async, no wait)
    send_render_command(RenderCommand::ReadComputeResult { shader_id: sid });

    // Clone receiver under lock, then release before spin-polling
    let rx = {
        let channels = COMPUTE_CHANNELS.get().unwrap();
        let guard = channels.lock().unwrap_or_else(|e| e.into_inner());
        guard.get(&sid).map(|(_, rx)| rx.clone())
    };

    if let Some(rx) = rx {
        for _ in 0..64 {
            match rx.try_recv() {
                Ok(floats) => {
                    return floats
                        .into_iter()
                        .map(|f| crate::executor::RelType::Float(f as f64))
                        .collect();
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    std::hint::spin_loop();
                }
                Err(crossbeam_channel::TryRecvError::Disconnected) => break,
            }
        }
    }
    vec![]
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

/// Sprint 170: Debug FFI function that intentionally panics to test the
/// panic-safety layer in the VM bridge. Must be caught by `catch_unwind`
/// in machine.rs — the application must NOT crash.
#[cfg(debug_assertions)]
pub fn registry_force_panic() {
    panic!("Simulated core dump from FFI!");
}

// Sprint 208: Asynchronous texture loading — I/O offloaded to background thread
pub fn registry_load_texture(path: &str) -> i64 {
    let safe_path = match crate::executor::ExecutionEngine::validate_fs_path(path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "[KnotenCore Texture] Security error loading '{}': {}",
                path, e
            );
            return 0;
        }
    };
    // Generate texture ID immediately — non-blocking
    let id = TEXTURE_ID_COUNTER.fetch_add(1, Ordering::Relaxed) as i64;
    let path_owned = path.to_string();

    std::thread::spawn(move || {
        let img_result = image::open(&safe_path);
        if let Ok(img) = img_result {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            let raw_data = rgba.into_raw();
            send_render_command(RenderCommand::LoadTexture {
                id: id as usize,
                width,
                height,
                rgba: raw_data,
            });
        } else {
            eprintln!("[KnotenCore Texture] Failed to load '{}'", path_owned);
            send_render_command(RenderCommand::LoadTextureFailed { id: id as usize });
        }
    });

    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_healing_module_reset() {
        registry_drain_failure_tracker();

        for _ in 0..4 {
            registry_track_failure("registry");
        }
        assert_eq!(registry_get_failure_count("registry"), 4);

        registry_track_failure("registry");
        assert_eq!(
            registry_get_failure_count("registry"),
            0,
            "Counter must reset after threshold (5 failures)"
        );

        registry_drain_failure_tracker();
    }

    #[test]
    fn test_agent_latency_tracking() {
        registry_drain_latency_data();

        registry_start_latency_timer("test_cmd");
        std::thread::sleep(std::time::Duration::from_millis(50));
        let elapsed = registry_stop_latency_timer("test_cmd");

        assert!(
            elapsed > 45_000 && elapsed < 60_000,
            "Elapsed {}us should be ~50ms (50000us)",
            elapsed
        );

        let avg = registry_get_avg_latency("test_cmd");
        assert!(
            avg > 45000.0 && avg < 60000.0,
            "Average latency {}us should be ~50ms",
            avg
        );

        registry_drain_latency_data();
    }

    #[test]
    fn test_lock_free_handle_concurrency() {
        let handle = registry_create_counter();

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let h = handle;
                std::thread::spawn(move || {
                    for _ in 0..20_000 {
                        registry_increment(h);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("Thread panicked");
        }

        assert_eq!(
            registry_get_value(handle),
            80_000,
            "4 threads × 20,000 increments must equal 80,000"
        );
    }
}
