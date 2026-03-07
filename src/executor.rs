use crate::ast::Node;
use crate::natives::NativeModule;
use crate::natives::bridge::{BridgeModule, CoreBridge};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use winit::event_loop::EventLoop;
use winit::window::Window;

#[derive(PartialEq)]
pub enum RelType {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Array(Vec<RelType>),
    Object(HashMap<String, RelType>),
    Handle(i64),
    FnDef(String, Vec<String>, Box<Node>),
    Call(String, Vec<Node>),
    Void,
}

#[derive(Clone)]
pub struct AgentPermissions {
    pub allow_network: bool,
    pub allowed_domains: Vec<String>,
    pub allow_fs_read: bool,
    pub allow_fs_write: bool,
}

impl Default for AgentPermissions {
    fn default() -> Self {
        Self { allow_network: false, allowed_domains: Vec::new(), allow_fs_read: false, allow_fs_write: false }
    }
}

impl Clone for RelType {
    fn clone(&self) -> Self {
        match self {
            RelType::Int(v) => RelType::Int(*v),
            RelType::Float(v) => RelType::Float(*v),
            RelType::Bool(v) => RelType::Bool(*v),
            RelType::Str(s) => RelType::Str(s.clone()),
            RelType::Array(a) => RelType::Array(a.clone()),
            RelType::Object(m) => RelType::Object(m.clone()),
            RelType::Handle(id) => { crate::natives::registry::registry_retain(*id); RelType::Handle(*id) }
            RelType::FnDef(a, b, c) => RelType::FnDef(a.clone(), b.clone(), c.clone()),
            RelType::Call(a, b) => RelType::Call(a.clone(), b.clone()),
            RelType::Void => RelType::Void,
        }
    }
}

impl std::fmt::Display for RelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelType::Int(v) => write!(f, "{}", v),
            RelType::Float(v) => if v.fract() == 0.0 && v.abs() < 1e15 { write!(f, "{:.1}", v) } else { write!(f, "{}", v) },
            RelType::Bool(v) => write!(f, "{}", v),
            RelType::Str(v) => write!(f, "{}", v),
            RelType::Array(v) => { let s: Vec<String> = v.iter().map(|i| i.to_string()).collect(); write!(f, "[{}]", s.join(", ")) }
            RelType::Object(map) => { let mut s = Vec::new(); for (k, v) in map { s.push(format!("{}: {}", k, v)); } write!(f, "{{{}}}", s.join(", ")) }
            RelType::Handle(id) => write!(f, "Handle<{}>", id),
            RelType::FnDef(_, _, _) => write!(f, "<Function>"),
            RelType::Call(_, _) => write!(f, "<Function Call>"),
            RelType::Void => write!(f, ""),
        }
    }
}

impl std::fmt::Debug for RelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self) }
}

#[derive(Clone, Copy)]
pub struct VoiceState {
    pub active: bool,
    pub freq: f32,
    pub waveform: u8,
    pub phase: f32,
}

impl Default for VoiceState {
    fn default() -> Self { VoiceState { active: false, freq: 440.0, waveform: 0, phase: 0.0 } }
}

pub struct MeshBuffers {
    pub vbo: wgpu::Buffer,
    pub ibo: wgpu::Buffer,
    pub index_count: u32,
}

pub struct StackFrame {
    pub locals: HashMap<String, RelType>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VoxelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VoxelInstance {
    pub instance_pos_and_id: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    pub transform: [[f32; 4]; 4],
    pub color_offset: [f32; 4],
    pub material_pbr: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct PointLightData {
    pub x: f32, pub y: f32, pub z: f32,
    pub r: f32, pub g: f32, pub b: f32,
    pub intensity: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLightStruct {
    pub pos: [f32; 4],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub material: [f32; 4],
    pub pbr: [f32; 4],
    pub camera_pos: [f32; 4],
    pub lights: [PointLightStruct; 4],
}

pub struct ExecutionEngine {
    pub memory: HashMap<String, RelType>,
    pub event_loop: Option<EventLoop<()>>,
    pub window: Option<Arc<Window>>,
    pub surface: Option<wgpu::Surface<'static>>,
    pub device: Option<wgpu::Device>,
    pub queue: Option<wgpu::Queue>,
    pub config: Option<wgpu::SurfaceConfiguration>,
    pub depth_texture_view: Option<wgpu::TextureView>,
    pub current_canvas_view: Option<wgpu::TextureView>,
    pub current_canvas_frame: Option<wgpu::SurfaceTexture>,
    pub default_texture_view: Option<wgpu::TextureView>,
    pub default_sampler: Option<wgpu::Sampler>,
    pub startup_time: std::time::Instant,
    pub shaders: Vec<wgpu::ShaderModule>,
    pub render_pipelines: HashMap<usize, wgpu::RenderPipeline>,
    pub native_modules: Vec<Box<dyn NativeModule>>,
    pub bridge: Box<dyn BridgeModule>,
    pub camera_active: bool,
    pub camera_pos: [f32; 3],
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    pub camera_fov: f32,
    pub input_w: bool, pub input_a: bool, pub input_s: bool, pub input_d: bool,
    pub input_space: bool, pub input_shift: bool, pub input_left_click: bool,
    pub interaction_active: bool,
    pub selected_voxel_pos: Option<[i64; 3]>,
    pub place_voxel_pos: Option<[i64; 3]>,
    pub voxel_pipeline: Option<wgpu::RenderPipeline>,
    pub voxel_vbo: Option<wgpu::Buffer>,
    pub voxel_ibo: Option<wgpu::Buffer>,
    pub voxel_instances: Vec<VoxelInstance>,
    pub voxel_bind_group: Option<wgpu::BindGroup>,
    pub voxel_atlas_bind_group: Option<wgpu::BindGroup>,
    pub voxel_ubo: Option<wgpu::Buffer>,
    pub voxel_map: HashMap<[i64; 3], u8>,
    pub voxel_map_active: bool,
    pub voxel_map_dirty: bool,
    pub interaction_enabled: bool,
    pub physics_enabled: bool,
    pub velocity_y: f32,
    pub is_grounded: bool,
    pub voxel_instance_buffer: Option<wgpu::Buffer>,
    pub meshes: Vec<MeshBuffers>,
    pub textures: Vec<(wgpu::Texture, wgpu::TextureView, wgpu::BindGroup, wgpu::BindGroupLayout)>,
    pub point_lights: Vec<PointLightData>,
    pub instance_queues: HashMap<i64, Vec<InstanceData>>,
    pub mouse_grab_enabled: bool,
    pub mouse_delta: (f32, f32),
    pub glyph_brush: Option<wgpu_glyph::GlyphBrush<()>>,
    pub staging_belt: Option<wgpu::util::StagingBelt>,
    pub keyboard_buffer: Arc<Mutex<String>>,
    pub egui_ctx: Option<egui::Context>,
    pub egui_state: Option<egui_winit::State>,
    pub egui_renderer: Option<egui_wgpu::Renderer>,
    pub egui_ui_ptr: Option<*mut egui::Ui>,
    pub voices: Option<Arc<Mutex<[VoiceState; 4]>>>,
    pub stream_samples: Option<Arc<Mutex<Vec<f32>>>>,
    pub stream_pos: Option<Arc<Mutex<usize>>>,
    pub audio_stream: Option<cpal::Stream>,
    pub audio_stream_handle: Option<(rodio::OutputStream, rodio::OutputStreamHandle)>,
    pub samples: HashMap<i64, std::sync::Arc<[u8]>>,
    pub async_bridge: Option<crate::async_bridge::AsyncBridge>,
    pub action_tx: Option<std::sync::mpsc::Sender<Action>>,
    pub action_rx: Option<std::sync::mpsc::Receiver<Action>>,
    pub permission_fault: Option<String>,
    pub ui_dirty: bool,
    pub permissions: AgentPermissions,
    pub call_stack: Vec<StackFrame>,
    pub render_canvas_active: bool,
    pub canvas_mesh_pipeline: Option<wgpu::RenderPipeline>,
    pub camera3d_view_proj: Option<[[f32; 4]; 4]>,
    pub canvas_material: [f32; 8],
    pub sprite2d_queue: Vec<(i64, f32, f32, f32, f32)>,
    pub transform2d_stack: Vec<[f32; 4]>,
    pub weapon_mesh: Option<i64>,
    pub weapon_tex: Option<i64>,
    pub weapon_sway: (f32, f32),
}

pub enum Action { UpdateData(String, RelType) }

pub enum ExecResult { Value(RelType), ReturnBlockInfo(RelType), Fault(String) }

impl std::fmt::Display for ExecResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecResult::Value(v) => write!(f, "{}", v),
            ExecResult::ReturnBlockInfo(v) => write!(f, "{}", v),
            ExecResult::Fault(e) => write!(f, "Fault: {}", e),
        }
    }
}

impl ExecutionEngine {
    pub fn new() -> Self {
        // ... (truncated for brevity, actual code below)
        Self::default_new()
    }

    pub fn execute(&mut self, node: &Node) -> ExecResult {
        self.evaluate(node)
    }

    pub fn get_var(&self, name: &str) -> Option<RelType> {
        for frame in self.call_stack.iter().rev() {
            if let Some(val) = frame.locals.get(name) { return Some(val.clone()); }
        }
        self.memory.get(name).cloned()
    }

    pub fn set_var(&mut self, name: String, val: RelType) {
        let mut old_val = None;
        for frame in self.call_stack.iter_mut().rev() {
            if frame.locals.contains_key(&name) {
                if let Some(v) = frame.locals.get(&name) { old_val = Some(v.clone()); }
                frame.locals.insert(name, val);
                if let Some(old) = old_val { self.release_handles(&old); }
                return;
            }
        }
        if let Some(frame) = self.call_stack.last_mut() {
            if let Some(v) = frame.locals.get(&name) { old_val = Some(v.clone()); }
            frame.locals.insert(name, val);
        } else {
            if let Some(v) = self.memory.get(&name) { old_val = Some(v.clone()); }
            self.memory.insert(name, val);
        }
        if let Some(old) = old_val { self.release_handles(&old); }
    }

    pub fn release_handles(&self, val: &RelType) {
        match val {
            RelType::Handle(id) => crate::natives::registry::registry_release(*id),
            RelType::Array(arr) => for i in arr { self.release_handles(i); },
            RelType::Object(map) => for v in map.values() { self.release_handles(v); },
            _ => {}
        }
    }

    fn default_new() -> Self {
        let mut engine = Self {
            memory: HashMap::new(), event_loop: None, window: None, surface: None,
            device: None, queue: None, config: None, depth_texture_view: None,
            shaders: Vec::new(), render_pipelines: HashMap::new(), native_modules: Vec::new(),
            camera_active: false, camera_pos: [0.0, 1.0, 0.0], camera_yaw: -90.0, camera_pitch: 0.0, camera_fov: 60.0,
            point_lights: Vec::new(), current_canvas_frame: None, current_canvas_view: None,
            default_texture_view: None, default_sampler: None, startup_time: std::time::Instant::now(),
            input_w: false, input_a: false, input_s: false, input_d: false, input_space: false, input_shift: false, input_left_click: false,
            interaction_active: false, selected_voxel_pos: None, place_voxel_pos: None, voxel_pipeline: None,
            voxel_vbo: None, voxel_ibo: None, voxel_instances: Vec::new(), voxel_bind_group: None, voxel_atlas_bind_group: None,
            voxel_ubo: None, voxel_map: HashMap::new(), voxel_map_active: false, voxel_map_dirty: false,
            interaction_enabled: false, physics_enabled: false, velocity_y: 0.0, is_grounded: false,
            voxel_instance_buffer: None, meshes: Vec::new(), textures: Vec::new(), instance_queues: HashMap::new(),
            mouse_grab_enabled: false, mouse_delta: (0.0, 0.0), glyph_brush: None, staging_belt: None,
            keyboard_buffer: Arc::new(Mutex::new(String::new())), egui_ctx: None, egui_state: None,
            egui_renderer: None, egui_ui_ptr: None, voices: None, stream_samples: None, stream_pos: None,
            audio_stream: None, audio_stream_handle: None, samples: HashMap::new(), async_bridge: None,
            action_tx: None, action_rx: None, permission_fault: None, ui_dirty: false,
            permissions: AgentPermissions::default(),
            call_stack: vec![StackFrame { locals: HashMap::new() }],
            render_canvas_active: false, canvas_mesh_pipeline: None, camera3d_view_proj: None,
            canvas_material: [1.0, 1.0, 1.0, 1.0, 0.0, 0.5, 0.0, 0.0],
            sprite2d_queue: Vec::new(), transform2d_stack: Vec::new(),
            weapon_mesh: None, weapon_tex: None, weapon_sway: (0.0, 0.0),
            bridge: Box::new(CoreBridge),
        };
        let (tx, rx) = std::sync::mpsc::channel();
        engine.action_tx = Some(tx);
        engine.action_rx = Some(rx);
        engine.native_modules.push(Box::new(crate::natives::math::MathModule));
        engine.native_modules.push(Box::new(crate::natives::io::IoModule));
        engine.native_modules.push(Box::new(crate::natives::registry::RegistryModule));
        engine
    }

    pub fn evaluate_extra(&mut self, node: &Node) -> ExecResult {
        match node {
            Node::PollEvents(body) => { self.run_event_loop(body); ExecResult::Value(RelType::Void) }
            Node::Mesh3D { primitive, .. } => {
               if let ExecResult::Value(RelType::Str(s)) = self.evaluate(primitive) {
                   self.draw_mesh_immediate(&s)
               } else { ExecResult::Fault("Invalid mesh primitive".into()) }
            }
            // ... more to come
            _ => ExecResult::Fault(format!("Unsupported node: {:?}", node)),
        }
    }
}
