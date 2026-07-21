use crate::natives::NativeModule;
use crate::natives::bridge::{BridgeModule, CoreBridge};
use knoten_core_types::ast::Node;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NativeHandle(pub i64);

impl Clone for NativeHandle {
    fn clone(&self) -> Self {
        crate::natives::registry::registry_retain(self.0);
        NativeHandle(self.0)
    }
}

impl Drop for NativeHandle {
    fn drop(&mut self) {
        crate::natives::registry::registry_release(self.0);
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum RelType {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Array(Vec<RelType>),
    Object(HashMap<String, RelType>),
    Dict(std::sync::Arc<std::sync::Mutex<HashMap<String, RelType>>>),
    Handle(NativeHandle),
    FnDef(String, Vec<String>, Box<Node>),
    Call(String, Vec<Node>),
    ASTNode(Box<Node>),
    Void,
}

impl PartialEq for RelType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RelType::Int(a), RelType::Int(b)) => a == b,
            (RelType::Float(a), RelType::Float(b)) => a == b,
            (RelType::Bool(a), RelType::Bool(b)) => a == b,
            (RelType::Str(a), RelType::Str(b)) => a == b,
            (RelType::Array(a), RelType::Array(b)) => a == b,
            (RelType::Object(a), RelType::Object(b)) => a == b,
            // Dict equality: same Arc pointer means same object
            (RelType::Dict(a), RelType::Dict(b)) => std::sync::Arc::ptr_eq(a, b),
            (RelType::ASTNode(a), RelType::ASTNode(b)) => a == b,
            (RelType::Void, RelType::Void) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Default)]
pub struct AgentPermissions {
    pub allow_network: bool,
    pub allowed_domains: Vec<String>,
    pub allow_fs_read: bool,
    pub allow_fs_write: bool,
}

impl std::fmt::Display for RelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelType::Int(v) => write!(f, "{}", v),
            RelType::Float(v) => {
                if v.fract() == 0.0 && v.abs() < 1e15 {
                    write!(f, "{:.1}", v)
                } else {
                    write!(f, "{}", v)
                }
            }
            RelType::Bool(v) => write!(f, "{}", v),
            RelType::Str(v) => write!(f, "{}", v),
            RelType::Array(v) => {
                let s: Vec<String> = v.iter().map(|i| i.to_string()).collect();
                write!(f, "[{}]", s.join(", "))
            }
            RelType::Object(map) => {
                let mut s = Vec::new();
                for (k, v) in map {
                    s.push(format!("{}: {}", k, v));
                }
                write!(f, "{{{}}}", s.join(", "))
            }
            RelType::Dict(map_arc) => {
                let map = map_arc.lock().unwrap_or_else(|e| e.into_inner());
                let mut s = Vec::new();
                for (k, v) in map.iter() {
                    s.push(format!("{}: {}", k, v));
                }
                write!(f, "{{{}}}", s.join(", "))
            }
            RelType::Handle(h) => write!(f, "<NativeHandle:{}>", h.0),
            RelType::FnDef(_, _, _) => write!(f, "<Function>"),
            RelType::Call(_, _) => write!(f, "<Function Call>"),
            RelType::ASTNode(_) => write!(f, "<AST Node>"),
            RelType::Void => write!(f, ""),
        }
    }
}

impl std::fmt::Debug for RelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Clone, Copy)]
pub struct VoiceState {
    pub active: bool,
    pub freq: f32,
    pub waveform: u8,
    pub phase: f32,
}

impl Default for VoiceState {
    fn default() -> Self {
        VoiceState {
            active: false,
            freq: 440.0,
            waveform: 0,
            phase: 0.0,
        }
    }
}

// Sprint 85: MeshBuffers removed — mesh/GPU resources are managed exclusively in window.rs (KnotenApp)

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
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
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
    pub startup_time: std::time::Instant,
    pub native_modules: Vec<Box<dyn NativeModule>>,
    pub bridge: Box<dyn BridgeModule>,
    // ── Camera / FPS state (read by executor nodes) ───────────────────
    pub camera_active: bool,
    pub camera_pos: [f32; 3],
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    pub camera_fov: f32,
    // ── Input state ──────────────────────────────────────────────────
    pub input_w: bool,
    pub input_a: bool,
    pub input_s: bool,
    pub input_d: bool,
    pub input_space: bool,
    pub input_shift: bool,
    pub input_left_click: bool,
    pub interaction_active: bool,
    pub interaction_enabled: bool,
    pub physics_enabled: bool,
    pub velocity_y: f32,
    pub is_grounded: bool,
    // ── Lighting (CPU-side, shared with renderer via camera UBO) ─────
    pub point_lights: Vec<PointLightData>,
    pub instance_queues: HashMap<i64, Vec<InstanceData>>,
    // ── Input / HID ──────────────────────────────────────────────────
    pub mouse_grab_enabled: bool,
    pub mouse_delta: (f32, f32),
    pub keyboard_buffer: Arc<Mutex<String>>,
    // ── Audio ────────────────────────────────────────────────────────
    pub voices: Option<Arc<Mutex<[VoiceState; 4]>>>,
    pub stream_samples: Option<Arc<Mutex<Vec<f32>>>>,
    pub stream_pos: Option<Arc<Mutex<usize>>>,
    pub audio_stream: Option<cpal::Stream>,
    pub audio_stream_handle: Option<(rodio::OutputStream, rodio::OutputStreamHandle)>,
    pub samples: HashMap<i64, std::sync::Arc<[u8]>>,
    // ── Async / Permissions / Actions ────────────────────────────────
    pub async_bridge: Option<crate::async_bridge::AsyncBridge>,
    pub action_tx: Option<std::sync::mpsc::Sender<Action>>,
    pub action_rx: Option<std::sync::mpsc::Receiver<Action>>,
    pub permission_fault: Option<String>,
    pub ui_dirty: bool,
    pub permissions: AgentPermissions,
    pub call_stack: Vec<StackFrame>,
    // ── 2D / Weapon ──────────────────────────────────────────────────
    pub render_canvas_active: bool,
    pub camera3d_view_proj: Option<[[f32; 4]; 4]>,
    pub canvas_material: [f32; 8],
    pub sprite2d_queue: Vec<(i64, f32, f32, f32, f32)>,
    pub transform2d_stack: Vec<[f32; 4]>,
    pub weapon_mesh: Option<i64>,
    pub weapon_tex: Option<i64>,
    pub weapon_sway: (f32, f32),
    // ── Physics AABBs ────────────────────────────────────────────────
    pub world_aabbs: Vec<crate::math::AABB>,
    pub camera_aabb_offset: crate::math::AABB,
    pub jit_accumulated_cpu: std::time::Duration,
}

// SAFETY: ExecutionEngine is moved to a background thread and stays there.
// The non-Send fields (audio streams) are only accessed by the engine itself.
// unsafe impl Sync is intentionally omitted: ExecutionEngine contains cpal::Stream
// which is !Sync. The engine is single-owner per thread and never shared across
// threads simultaneously, so Send alone is sufficient.
unsafe impl Send for ExecutionEngine {}

pub enum Action {
    UpdateData(String, RelType),
}

pub enum ExecResult {
    Value(RelType),
    ReturnBlockInfo(RelType),
    Fault { msg: String, node: String },
}

impl std::fmt::Display for ExecResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecResult::Value(v) => write!(f, "{}", v),
            ExecResult::ReturnBlockInfo(v) => write!(f, "{}", v),
            ExecResult::Fault { msg, node } => write!(f, "Fault: {} (at {})", msg, node),
        }
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
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

    pub fn poll_async_bridge(&mut self) {
        let mut payloads = Vec::new();
        if let Some(bridge) = &self.async_bridge {
            while let Some(payload) = bridge.try_recv() {
                payloads.push(payload);
            }
        }
        for payload in payloads {
            let (data, is_err) = match payload.payload {
                Ok(s) => (RelType::Str(s), RelType::Bool(false)),
                Err(e) => (RelType::Str(e), RelType::Bool(true)),
            };
            self.memory.insert("fetch_result".into(), data);
            self.memory.insert("fetch_error".into(), is_err);
            let _ = self.evaluate(&payload.callback_node);
        }
    }

    pub fn get_var(&self, name: &str) -> Option<RelType> {
        for frame in self.call_stack.iter().rev() {
            if let Some(val) = frame.locals.get(name) {
                return Some(val.clone());
            }
        }
        self.memory.get(name).cloned()
    }

    pub fn set_var(&mut self, name: String, val: RelType) {
        // Walk the call stack from innermost → outermost looking for an existing binding.
        // If found, update in place (assignment to an already-declared variable).
        for frame in self.call_stack.iter_mut().rev() {
            if let Some(v) = frame.locals.get_mut(&name) {
                *v = val;
                return;
            }
        }
        // FINDING-09 FIX: Variable not found in any frame → it is a new global declaration.
        // Always create new variables in self.memory, not in the innermost call frame.
        // This prevents silent scoping bugs where top-level variables defined inside a
        // function call would be garbage-collected when the function's frame is popped.
        self.memory.insert(name, val);
    }

    /// Sprint 88 Optimization: In-place Map Insertion
    /// Avoids cloning the entire HashMap when setting or adding a single key
    pub fn mutate_map_insert(
        &mut self,
        name: &str,
        key: String,
        val: RelType,
    ) -> Result<Option<RelType>, ExecResult> {
        for frame in self.call_stack.iter_mut().rev() {
            if let Some(RelType::Object(m)) = frame.locals.get_mut(name) {
                return Ok(m.insert(key, val));
            } else if frame.locals.contains_key(name) {
                return Err(ExecResult::Fault {
                    msg: "Target is not a map/object".into(),
                    node: "Node::MapSet/PropertySet".into(),
                });
            }
        }
        if let Some(RelType::Object(m)) = self.memory.get_mut(name) {
            return Ok(m.insert(key, val));
        }
        Err(ExecResult::Fault {
            msg: "Target is not a map/object".into(),
            node: "Node::MapSet/PropertySet".into(),
        })
    }

    /// Sprint 88 Optimization: In-place Array Modification
    /// Avoids cloning the entire Vec when setting a single index
    pub fn mutate_array_set(
        &mut self,
        name: &str,
        idx: usize,
        val: RelType,
    ) -> Result<RelType, ExecResult> {
        for frame in self.call_stack.iter_mut().rev() {
            if let Some(RelType::Array(a)) = frame.locals.get_mut(name) {
                if idx < a.len() {
                    return Ok(std::mem::replace(&mut a[idx], val));
                }
                return Err(ExecResult::Fault {
                    msg: format!("Index {} out of bounds", idx),
                    node: "Node::ArraySet".into(),
                });
            } else if frame.locals.contains_key(name) {
                return Err(ExecResult::Fault {
                    msg: "Target is not an array".into(),
                    node: "Node::ArraySet".into(),
                });
            }
        }
        if let Some(RelType::Array(a)) = self.memory.get_mut(name) {
            if idx < a.len() {
                return Ok(std::mem::replace(&mut a[idx], val));
            }
            return Err(ExecResult::Fault {
                msg: format!("Index {} out of bounds", idx),
                node: "Node::ArraySet".into(),
            });
        }
        Err(ExecResult::Fault {
            msg: "Target is not an array".into(),
            node: "Node::ArraySet".into(),
        })
    }

    /// Sprint 88 Optimization: In-place Array Push
    /// Avoids cloning the entire Vec when pushing a single item
    pub fn mutate_array_push(&mut self, name: &str, val: RelType) -> Result<(), ExecResult> {
        for frame in self.call_stack.iter_mut().rev() {
            if let Some(RelType::Array(a)) = frame.locals.get_mut(name) {
                a.push(val);
                return Ok(());
            } else if frame.locals.contains_key(name) {
                return Err(ExecResult::Fault {
                    msg: "Target is not an array".into(),
                    node: "Node::ArrayPush".into(),
                });
            }
        }
        if let Some(RelType::Array(a)) = self.memory.get_mut(name) {
            a.push(val);
            return Ok(());
        }
        Err(ExecResult::Fault {
            msg: "Target is not an array".into(),
            node: "Node::ArrayPush".into(),
        })
    }

    pub fn release_handles(&self, _val: &RelType) {
        // FINDING-01 ANALYSIS: This is intentionally a no-op.
        // NativeHandle implements Drop, which calls registry_release automatically.
        // FnDef(_, _, Box<Node>) is freed by Rust's drop glue when the RelType value
        // goes out of scope or is overwritten via set_var.
        // Re-entering this function to manually recurse would double-count releases
        // for NativeHandle variants. The call sites in evaluator.rs (Block, While,
        // Call frame cleanup) ensure values are dropped immediately after this call.
    }

    fn default_new() -> Self {
        let mut engine = Self {
            memory: HashMap::new(),
            startup_time: std::time::Instant::now(),
            native_modules: Vec::new(),
            bridge: Box::new(CoreBridge),
            camera_active: false,
            camera_pos: [0.0, 1.0, 0.0],
            camera_yaw: -90.0,
            camera_pitch: 0.0,
            camera_fov: 60.0,
            input_w: false,
            input_a: false,
            input_s: false,
            input_d: false,
            input_space: false,
            input_shift: false,
            input_left_click: false,
            interaction_active: false,
            interaction_enabled: false,
            physics_enabled: false,
            velocity_y: 0.0,
            is_grounded: false,
            point_lights: Vec::new(),
            instance_queues: HashMap::new(),
            mouse_grab_enabled: false,
            mouse_delta: (0.0, 0.0),
            keyboard_buffer: Arc::new(Mutex::new(String::new())),
            voices: None,
            stream_samples: None,
            stream_pos: None,
            audio_stream: None,
            audio_stream_handle: None,
            samples: HashMap::new(),
            async_bridge: Some(crate::async_bridge::AsyncBridge::new()),
            action_tx: None,
            action_rx: None,
            permission_fault: None,
            ui_dirty: false,
            permissions: AgentPermissions::default(),
            call_stack: vec![StackFrame {
                locals: HashMap::new(),
            }],
            render_canvas_active: false,
            camera3d_view_proj: None,
            canvas_material: [1.0, 1.0, 1.0, 1.0, 0.0, 0.5, 0.0, 0.0],
            sprite2d_queue: Vec::new(),
            transform2d_stack: Vec::new(),
            weapon_mesh: None,
            weapon_tex: None,
            weapon_sway: (0.0, 0.0),
            world_aabbs: Vec::new(),
            camera_aabb_offset: crate::math::AABB::new([-0.3, -1.6, -0.3], [0.3, 0.2, 0.3]),
            jit_accumulated_cpu: std::time::Duration::ZERO,
        };
        let (tx, rx) = std::sync::mpsc::channel();
        engine.action_tx = Some(tx);
        engine.action_rx = Some(rx);
        engine
            .native_modules
            .push(Box::new(crate::natives::math::MathModule));
        engine
            .native_modules
            .push(Box::new(crate::natives::io::IoModule));
        engine
            .native_modules
            .push(Box::new(crate::natives::registry::RegistryModule));
        engine
    }

    pub fn evaluate_extra(&mut self, node: &Node) -> ExecResult {
        match node {
            Node::PollEvents(body) => self.evaluate(body),
            Node::Print(expr) => match self.evaluate(expr) {
                ExecResult::Value(v) => {
                    println!("{}", v);
                    ExecResult::Value(RelType::Void)
                }
                err => err,
            },
            Node::Mesh3D {
                primitive: _,
                material: _,
            } => {
                println!(
                    "Warning: Node::Mesh3D is deprecated in Sprint 82. Use native primitives like Cube, Sphere, etc."
                );
                ExecResult::Value(RelType::Void)
            }
            Node::PointLight3D {
                x,
                y,
                z,
                r,
                g,
                b,
                intensity,
            } => {
                let px = match self.evaluate(x) {
                    ExecResult::Value(RelType::Float(f)) => f as f32,
                    ExecResult::Value(RelType::Int(i)) => i as f32,
                    _ => 0.0,
                };
                let py = match self.evaluate(y) {
                    ExecResult::Value(RelType::Float(f)) => f as f32,
                    ExecResult::Value(RelType::Int(i)) => i as f32,
                    _ => 0.0,
                };
                let pz = match self.evaluate(z) {
                    ExecResult::Value(RelType::Float(f)) => f as f32,
                    ExecResult::Value(RelType::Int(i)) => i as f32,
                    _ => 0.0,
                };
                let cr = match self.evaluate(r) {
                    ExecResult::Value(RelType::Float(f)) => f as f32,
                    ExecResult::Value(RelType::Int(i)) => i as f32,
                    _ => 1.0,
                };
                let cg = match self.evaluate(g) {
                    ExecResult::Value(RelType::Float(f)) => f as f32,
                    ExecResult::Value(RelType::Int(i)) => i as f32,
                    _ => 1.0,
                };
                let cb = match self.evaluate(b) {
                    ExecResult::Value(RelType::Float(f)) => f as f32,
                    ExecResult::Value(RelType::Int(i)) => i as f32,
                    _ => 1.0,
                };
                let intens = match self.evaluate(intensity) {
                    ExecResult::Value(RelType::Float(f)) => f as f32,
                    ExecResult::Value(RelType::Int(i)) => i as f32,
                    _ => 1.0,
                };
                self.point_lights.push(PointLightData {
                    x: px,
                    y: py,
                    z: pz,
                    r: cr,
                    g: cg,
                    b: cb,
                    intensity: intens,
                });
                ExecResult::Value(RelType::Void)
            }
            Node::InitGraphics => {
                self.interaction_enabled = true;
                ExecResult::Value(RelType::Void)
            }
            Node::InitAudio => ExecResult::Value(RelType::Void),
            Node::AddWorldAABB { min, max } => {
                let v_min = match self.evaluate(min) {
                    ExecResult::Value(v) => self.to_vec3(v),
                    _ => None,
                };
                let v_max = match self.evaluate(max) {
                    ExecResult::Value(v) => self.to_vec3(v),
                    _ => None,
                };
                if let (Some(mi), Some(ma)) = (v_min, v_max) {
                    self.world_aabbs.push(crate::math::AABB::new(mi, ma));
                    ExecResult::Value(RelType::Void)
                } else {
                    ExecResult::Fault {
                        msg: "AddWorldAABB expects two arrays of 3 floats".into(),
                        node: "Node::AddWorldAABB".into(),
                    }
                }
            }
            Node::MouseGrab { enabled } => {
                if let ExecResult::Value(RelType::Bool(v)) = self.evaluate(enabled) {
                    self.mouse_grab_enabled = v;
                }
                ExecResult::Value(RelType::Void)
            }
            Node::FPSCamera { fov } => {
                if let ExecResult::Value(RelType::Float(f)) = self.evaluate(fov) {
                    self.camera_fov = f as f32;
                    self.camera_active = true;
                }
                ExecResult::Value(RelType::Void)
            }
            Node::WeaponViewModel { mesh, tex } => {
                if let ExecResult::Value(RelType::Int(m)) = self.evaluate(mesh) {
                    self.weapon_mesh = Some(m);
                }
                if let ExecResult::Value(RelType::Int(t)) = self.evaluate(tex) {
                    self.weapon_tex = Some(t);
                }
                ExecResult::Value(RelType::Void)
            }
            Node::Store { key, value } => {
                if let ExecResult::Value(v) = self.evaluate(value) {
                    self.memory.insert(key.clone(), v);
                }
                ExecResult::Value(RelType::Void)
            }
            Node::Load { key } => {
                if let Some(v) = self.memory.get(key) {
                    ExecResult::Value(v.clone())
                } else {
                    ExecResult::Value(RelType::Void)
                }
            }
            Node::FileRead(path) => {
                if !self.permissions.allow_fs_read {
                    return ExecResult::Fault {
                        msg: "Permission Denied: allow_fs_read is false".into(),
                        node: "Node::FileRead".into(),
                    };
                }
                if let ExecResult::Value(RelType::Str(p)) = self.evaluate(path) {
                    // FINDING-05: Canonicalize path to prevent directory traversal escapes
                    match Self::validate_fs_path(&p) {
                        Err(e) => ExecResult::Fault {
                            msg: format!("Security: {}", e),
                            node: "Node::FileRead".into(),
                        },
                        Ok(safe_path) => match std::fs::read_to_string(&safe_path) {
                            Ok(s) => ExecResult::Value(RelType::Str(s)),
                            Err(e) => ExecResult::Fault {
                                msg: format!("File read error: {}", e),
                                node: "Node::FileRead".into(),
                            },
                        },
                    }
                } else {
                    ExecResult::Fault {
                        msg: "FileRead expects string path".into(),
                        node: "Node::FileRead".into(),
                    }
                }
            }
            Node::FileWrite(path, data) => {
                if !self.permissions.allow_fs_write {
                    return ExecResult::Fault {
                        msg: "Permission Denied: allow_fs_write is false".into(),
                        node: "Node::FileWrite".into(),
                    };
                }
                if let (ExecResult::Value(RelType::Str(p)), ExecResult::Value(RelType::Str(d))) =
                    (self.evaluate(path), self.evaluate(data))
                {
                    // FINDING-05: Canonicalize path to prevent directory traversal escapes
                    match Self::validate_fs_path_write(&p) {
                        Err(e) => ExecResult::Fault {
                            msg: format!("Security: {}", e),
                            node: "Node::FileWrite".into(),
                        },
                        Ok(safe_path) => {
                            if let Err(e) = std::fs::write(&safe_path, &d) {
                                return ExecResult::Fault {
                                    msg: format!("File write error: {}", e),
                                    node: "Node::FileWrite".into(),
                                };
                            }
                            ExecResult::Value(RelType::Void)
                        }
                    }
                } else {
                    ExecResult::Fault {
                        msg: "FileWrite expects string path and data".into(),
                        node: "Node::FileWrite".into(),
                    }
                }
            }
            Node::FSRead(path) => {
                if !self.permissions.allow_fs_read {
                    return ExecResult::Fault {
                        msg: "Permission Denied: allow_fs_read is false".into(),
                        node: "Node::FSRead".into(),
                    };
                }
                if let ExecResult::Value(RelType::Str(p)) = self.evaluate(path) {
                    // FINDING-05: Canonicalize path to prevent directory traversal escapes
                    match Self::validate_fs_path(&p) {
                        Err(e) => ExecResult::Fault {
                            msg: format!("Security: {}", e),
                            node: "Node::FSRead".into(),
                        },
                        Ok(safe_path) => match std::fs::read_to_string(&safe_path) {
                            Ok(s) => ExecResult::Value(RelType::Str(s)),
                            Err(e) => ExecResult::Fault {
                                msg: format!("FSRead error: {}", e),
                                node: "Node::FSRead".into(),
                            },
                        },
                    }
                } else {
                    ExecResult::Fault {
                        msg: "FSRead expects string path".into(),
                        node: "Node::FSRead".into(),
                    }
                }
            }
            Node::FSWrite(path, data) => {
                if !self.permissions.allow_fs_write {
                    return ExecResult::Fault {
                        msg: "Permission Denied: allow_fs_write is false".into(),
                        node: "Node::FSWrite".into(),
                    };
                }
                if let (ExecResult::Value(RelType::Str(p)), ExecResult::Value(RelType::Str(d))) =
                    (self.evaluate(path), self.evaluate(data))
                {
                    // FINDING-05: Canonicalize path to prevent directory traversal escapes
                    match Self::validate_fs_path_write(&p) {
                        Err(e) => ExecResult::Fault {
                            msg: format!("Security: {}", e),
                            node: "Node::FSWrite".into(),
                        },
                        Ok(safe_path) => {
                            if let Err(e) = std::fs::write(&safe_path, &d) {
                                return ExecResult::Fault {
                                    msg: format!("FSWrite error: {}", e),
                                    node: "Node::FSWrite".into(),
                                };
                            }
                            ExecResult::Value(RelType::Void)
                        }
                    }
                } else {
                    ExecResult::Fault {
                        msg: "FSWrite expects string path and data".into(),
                        node: "Node::FSWrite".into(),
                    }
                }
            }
            Node::NativeCall(name, args) => {
                let mut v_args = Vec::with_capacity(args.len());
                for a in args {
                    match self.evaluate(a) {
                        ExecResult::Value(v) => v_args.push(v),
                        err => return err,
                    }
                }
                for mod_ in &self.native_modules {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        mod_.handle(name, &v_args, &self.permissions)
                    }));
                    match result {
                        Ok(Some(res)) => return res,
                        Ok(None) => {}
                        Err(panic_payload) => {
                            let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                                s.to_string()
                            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                                s.clone()
                            } else {
                                "Unknown FFI panic".to_string()
                            };
                            return ExecResult::Fault {
                                msg: format!("Native FFI Panic: {}", msg),
                                node: "Node::NativeCall".into(),
                            };
                        }
                    }
                }
                ExecResult::Fault {
                    msg: format!("Native function '{}' not found", name),
                    node: "Node::NativeCall".into(),
                }
            }
            Node::ExternCall {
                module,
                function,
                args,
            } => {
                let mut v_args = Vec::with_capacity(args.len());
                for a in args {
                    match self.evaluate(a) {
                        ExecResult::Value(v) => v_args.push(v),
                        err => return err,
                    }
                }

                // Security Lockdown: Intercept sensitive ExternCalls before they hit the bridge
                if module == "fs" || module == "registry" {
                    // Strict whitelist of functions requiring READ permissions
                    let read_requires = [
                        "registry_texture_load", // Loading a texture reads a file
                        "registry_play_sound",
                        "registry_loop_music",
                        "fs_read",
                        "fs_exists",
                    ];

                    // Strict whitelist of functions requiring WRITE permissions
                    let write_requires =
                        ["registry_file_create", "fs_write", "fs_create", "fs_append"];

                    if (read_requires.contains(&function.as_str())
                        && !self.permissions.allow_fs_read)
                        || (write_requires.contains(&function.as_str())
                            && !self.permissions.allow_fs_write)
                    {
                        let permission_type = if read_requires.contains(&function.as_str()) {
                            "FS_READ"
                        } else {
                            "FS_WRITE"
                        };
                        return ExecResult::Fault {
                            msg: format!(
                                "Permission Denied: {} required for {}.{}",
                                permission_type, module, function
                            ),
                            node: "Node::ExternCall".into(),
                        };
                    }
                }

                if module == "registry" && function == "registry_raycast_aabb" {
                    if v_args.len() == 2
                        && let RelType::Array(origin) = &v_args[0]
                        && let RelType::Array(dir) = &v_args[1]
                        && origin.len() >= 3
                        && dir.len() >= 3
                    {
                        let get_f = |val: &RelType| -> f32 {
                            match val {
                                RelType::Float(f) => *f as f32,
                                RelType::Int(i) => *i as f32,
                                _ => 0.0,
                            }
                        };
                        let ox = get_f(&origin[0]);
                        let oy = get_f(&origin[1]);
                        let oz = get_f(&origin[2]);

                        let dx = get_f(&dir[0]);
                        let dy = get_f(&dir[1]);
                        let dz = get_f(&dir[2]);

                        let ray_origin = glam::Vec3::new(ox, oy, oz);
                        let ray_dir = glam::Vec3::new(dx, dy, dz);

                        let mut hit_idx: i64 = -1;
                        let mut t_min = f32::MAX;
                        for (idx, aabb) in self.world_aabbs.iter().enumerate() {
                            if let Some(t) = aabb.intersect_ray(ray_origin, ray_dir)
                                && t < t_min
                            {
                                t_min = t;
                                hit_idx = idx as i64;
                            }
                        }
                        return ExecResult::Value(RelType::Int(hit_idx));
                    }
                    return ExecResult::Fault {
                        msg: "[FFI] registry_raycast_aabb expects (Array, Array) of size >= 3"
                            .to_string(),
                        node: "Node::ExternCall".into(),
                    };
                }

                if module == "registry" {
                    if function == "registry_play_sound" {
                        if v_args.len() == 1
                            && let RelType::Str(path) = &v_args[0]
                        {
                            match Self::validate_fs_path(path) {
                                Ok(safe_path) => {
                                    crate::natives::registry::init_audio_state();
                                    if let Ok(mut lock) =
                                        crate::natives::registry::AUDIO_STATE.lock()
                                        && let Some(audio) = lock.as_mut()
                                    {
                                        let _ = audio.play_sound(&safe_path.to_string_lossy());
                                    }
                                    return ExecResult::Value(RelType::Void);
                                }
                                Err(e) => {
                                    return ExecResult::Fault {
                                        msg: e,
                                        node: "Node::ExternCall".into(),
                                    };
                                }
                            }
                        }
                        return ExecResult::Fault {
                            msg: "registry_play_sound expects (String)".to_string(),
                            node: "Node::ExternCall".into(),
                        };
                    }
                    if function == "registry_loop_music" {
                        if v_args.len() == 1
                            && let RelType::Str(path) = &v_args[0]
                        {
                            match Self::validate_fs_path(path) {
                                Ok(safe_path) => {
                                    crate::natives::registry::init_audio_state();
                                    if let Ok(mut lock) =
                                        crate::natives::registry::AUDIO_STATE.lock()
                                        && let Some(audio) = lock.as_mut()
                                    {
                                        let _ = audio.loop_music(&safe_path.to_string_lossy());
                                    }
                                    return ExecResult::Value(RelType::Void);
                                }
                                Err(e) => {
                                    return ExecResult::Fault {
                                        msg: e,
                                        node: "Node::ExternCall".into(),
                                    };
                                }
                            }
                        }
                        return ExecResult::Fault {
                            msg: "registry_loop_music expects (String)".to_string(),
                            node: "Node::ExternCall".into(),
                        };
                    }
                    if function == "registry_set_volume" {
                        if v_args.len() == 1 {
                            let level = match &v_args[0] {
                                RelType::Float(f) => *f as f32,
                                RelType::Int(i) => *i as f32,
                                _ => {
                                    return ExecResult::Fault {
                                        msg: "registry_set_volume expects (Float/Int)".to_string(),
                                        node: "Node::ExternCall".into(),
                                    };
                                }
                            };
                            crate::natives::registry::init_audio_state();
                            if let Ok(mut lock) = crate::natives::registry::AUDIO_STATE.lock()
                                && let Some(audio) = lock.as_mut()
                            {
                                audio.set_volume(level);
                            }
                            return ExecResult::Value(RelType::Void);
                        }
                        return ExecResult::Fault {
                            msg: "registry_set_volume expects (Float/Int)".to_string(),
                            node: "Node::ExternCall".into(),
                        };
                    }
                }

                let bridge_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.bridge
                        .handle(module, function, &v_args, &self.permissions)
                }));
                match bridge_result {
                    Ok(Some(res)) => return res,
                    Ok(None) => {}
                    Err(panic_payload) => {
                        let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                            s.to_string()
                        } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "Unknown FFI panic".to_string()
                        };
                        return ExecResult::Fault {
                            msg: format!("ExternCall FFI Panic: {}", msg),
                            node: "Node::ExternCall".into(),
                        };
                    }
                }
                ExecResult::Fault {
                    msg: format!("Extern function '{}.{}' not found", module, function),
                    node: "Node::ExternCall".into(),
                }
            }
            Node::UIWindow(_id, _title, body) => {
                let mut ui_nodes = Vec::new();
                match &**body {
                    Node::Block(statements) => {
                        for stmt in statements {
                            ui_nodes.push(stmt.clone());
                        }
                    }
                    _ => ui_nodes.push(*body.clone()),
                }
                ui_nodes = self.resolve_ui_nodes(ui_nodes);
                crate::natives::registry::send_ui_nodes(ui_nodes);
                self.evaluate(body)
            }
            Node::UIButton(_text) => ExecResult::Value(RelType::Bool(false)),
            Node::UILabel(text) => {
                self.evaluate(text);
                ExecResult::Value(RelType::Void)
            }
            Node::UITextInput(initial) => {
                // Sprint 118: State-binding pattern — text = UITextInput(text)
                // Seed the buffer from the script variable on the first call (when buffer is empty).
                // Thereafter, return whatever is in the shared buffer
                // (which egui will write into on each frame via ui_text_input_set).
                if let ExecResult::Value(RelType::Str(seed)) = self.evaluate(initial) {
                    let current = crate::natives::ui::ui_text_input_get();
                    if current.is_empty() && !seed.is_empty() {
                        crate::natives::ui::ui_text_input_set(seed);
                    }
                }
                ExecResult::Value(RelType::Str(crate::natives::ui::ui_text_input_get()))
            }
            Node::UISetStyle(rounding, spacing, accent, fill, btn_idle, btn_hover) => {
                let r_val = match self.evaluate(rounding) {
                    ExecResult::Value(RelType::Float(f)) => f as f32,
                    ExecResult::Value(RelType::Int(i)) => i as f32,
                    _ => 0.0,
                };
                let s_val = match self.evaluate(spacing) {
                    ExecResult::Value(RelType::Float(f)) => f as f32,
                    ExecResult::Value(RelType::Int(i)) => i as f32,
                    _ => 0.0,
                };

                fn parse_rgba_jit(val: &RelType) -> Option<[f32; 4]> {
                    if let RelType::Array(arr) = val
                        && arr.len() >= 4
                    {
                        let r = match arr[0] {
                            RelType::Float(f) => f as f32,
                            RelType::Int(i) => i as f32,
                            _ => 0.0,
                        };
                        let g = match arr[1] {
                            RelType::Float(f) => f as f32,
                            RelType::Int(i) => i as f32,
                            _ => 0.0,
                        };
                        let b = match arr[2] {
                            RelType::Float(f) => f as f32,
                            RelType::Int(i) => i as f32,
                            _ => 0.0,
                        };
                        let a = match arr[3] {
                            RelType::Float(f) => f as f32,
                            RelType::Int(i) => i as f32,
                            _ => 1.0,
                        };
                        return Some([r, g, b, a]);
                    }
                    None
                }

                let acc_rgba = match self.evaluate(accent) {
                    ExecResult::Value(ref val) => {
                        parse_rgba_jit(val).unwrap_or([0.1, 0.5, 0.9, 1.0])
                    }
                    _ => [0.1, 0.5, 0.9, 1.0],
                };
                let fill_rgba = match self.evaluate(fill) {
                    ExecResult::Value(ref val) => {
                        parse_rgba_jit(val).unwrap_or([0.1, 0.1, 0.1, 1.0])
                    }
                    _ => [0.1, 0.1, 0.1, 1.0],
                };

                let idle_rgba = if let Some(idle_node) = btn_idle {
                    match self.evaluate(idle_node) {
                        ExecResult::Value(ref val) => parse_rgba_jit(val),
                        _ => None,
                    }
                } else {
                    None
                };

                let hover_rgba = if let Some(hover_node) = btn_hover {
                    match self.evaluate(hover_node) {
                        ExecResult::Value(ref val) => parse_rgba_jit(val),
                        _ => None,
                    }
                } else {
                    None
                };

                crate::natives::registry::send_render_command(
                    crate::natives::registry::RenderCommand::UpdateStyle {
                        window_id: 1,
                        rounding: r_val,
                        spacing: s_val,
                        accent_rgba: acc_rgba,
                        fill_rgba,
                        btn_idle_rgba: idle_rgba,
                        btn_hover_rgba: hover_rgba,
                    },
                );

                ExecResult::Value(RelType::Void)
            }
            Node::UIHorizontal(body)
            | Node::UIFullscreen(body)
            | Node::UIGrid(_, _, body)
            | Node::UIScrollArea(_, body) => self.evaluate(body),
            Node::UISplitPanel {
                left_body,
                right_body,
                ..
            } => {
                self.evaluate(left_body);
                self.evaluate(right_body)
            }
            Node::UIHBox(children) | Node::UIVBox(children) => {
                for child in children {
                    self.evaluate(child);
                }
                ExecResult::Value(RelType::Void)
            }
            Node::UIFixed { body, .. } => self.evaluate(body),
            Node::UIFillParent => ExecResult::Value(RelType::Void),
            Node::Fetch {
                method,
                url,
                callback,
            } => {
                // FINDING-03 FIX: Check network permission before dispatching fetch
                if !self.permissions.allow_network {
                    return ExecResult::Fault {
                        msg: "Permission Denied: allow_network is false. Use --allow-network flag."
                            .into(),
                        node: "Node::Fetch".into(),
                    };
                }
                if let Some(bridge) = &self.async_bridge {
                    bridge.dispatch_fetch(method.clone(), url.clone(), callback.clone());
                    ExecResult::Value(RelType::Void)
                } else {
                    ExecResult::Fault {
                        msg: "AsyncBridge not initialized".into(),
                        node: "Node::Fetch".into(),
                    }
                }
            }
            Node::Extract { .. } => ExecResult::Fault {
                msg: "Extract not implemented".into(),
                node: "Node::Extract".into(),
            },
            Node::EvalJSONNative(json_expr) => {
                if let ExecResult::Value(RelType::Str(json)) = self.evaluate(json_expr) {
                    match crate::natives::fs::fs_parse_json(&json) {
                        Ok(r) => ExecResult::Value(r),
                        Err(e) => ExecResult::Fault {
                            msg: format!("JSON Parse error: {}", e),
                            node: "Node::EvalJSONNative".into(),
                        },
                    }
                } else {
                    ExecResult::Fault {
                        msg: "EvalJSONNative expects string".into(),
                        node: "Node::EvalJSONNative".into(),
                    }
                }
            }
            Node::ToString(expr) => {
                ExecResult::Value(RelType::Str(self.evaluate(expr).to_string()))
            }
            Node::Import(_p) => ExecResult::Value(RelType::Void),
            Node::GetLastKeypress => ExecResult::Value(RelType::Str("".into())),
            Node::DrawRect { .. } => ExecResult::Value(RelType::Void),
            Node::RenderCanvas { body } => self.evaluate(body),
            Node::Transform2D { body, .. } => self.evaluate(body),
            Node::Sprite2D { .. } => ExecResult::Value(RelType::Void),
            Node::Camera3D { .. } => {
                self.camera_active = true;
                ExecResult::Value(RelType::Void)
            }
            Node::Material3D { .. } => ExecResult::Value(RelType::Void),
            Node::MeshInstance3D { .. } => ExecResult::Value(RelType::Void),
            Node::RaycastSimple => ExecResult::Value(RelType::Void),
            Node::InitWindow(_, _, _)
            | Node::LoadShader(_)
            | Node::RenderMesh(_, _, _)
            | Node::LoadComputeShader(_)
            | Node::DispatchCompute { .. }
            | Node::DispatchComputeLoop { .. } => ExecResult::Value(RelType::Void),
            Node::LoadMesh(_) | Node::LoadTexture(_) | Node::RenderAsset(_, _, _, _) => {
                ExecResult::Value(RelType::Void)
            }
            Node::LoadFont(_) | Node::DrawText(_, _, _, _, _) => ExecResult::Value(RelType::Void),
            Node::PlayNote(_, _, _, _) | Node::StopNote(_) | Node::PlayAudioFile(_) => {
                ExecResult::Value(RelType::Void)
            }
            Node::LoadSample(_, _) | Node::PlaySample(_, _, _) => ExecResult::Value(RelType::Void),
            Node::StructDef { .. } | Node::StructCreate { .. } => ExecResult::Value(RelType::Void),
            Node::StructFieldSet { obj, field, value } => match self.evaluate(obj) {
                ExecResult::Value(RelType::Object(mut map)) => match self.evaluate(value) {
                    ExecResult::Value(v) => {
                        map.insert(field.clone(), v);
                        ExecResult::Value(RelType::Object(map))
                    }
                    e => e,
                },
                ExecResult::Value(RelType::Dict(arc)) => match self.evaluate(value) {
                    ExecResult::Value(v) => {
                        arc.lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .insert(field.clone(), v);
                        ExecResult::Value(RelType::Dict(arc))
                    }
                    e => e,
                },
                ExecResult::Value(v) => ExecResult::Fault {
                    msg: format!(
                        "StructFieldSet: expected Object or Dict target, got {:?}",
                        v
                    ),
                    node: "StructFieldSet".into(),
                },
                e @ ExecResult::Fault { .. } => e,
                _ => ExecResult::Value(RelType::Void),
            },
            _ => ExecResult::Fault {
                msg: format!("Unsupported node in executor: {:?}", node),
                node: "Executor".into(),
            },
        }
    }
}

impl ExecutionEngine {
    /// FINDING-05: Validate and canonicalize a filesystem path for read operations.
    /// The resolved path must be a descendant of the current working directory.
    /// Sprint 190: Symlink blocking — rejects any path containing a symbolic link.
    pub fn validate_fs_path(path: &str) -> Result<PathBuf, String> {
        let cwd = std::env::current_dir()
            .map_err(|e| format!("Cannot determine working directory: {}", e))?;
        // Sprint 190: Walk each path component and reject symlinks
        let path_obj = std::path::Path::new(path);
        let check_path = if path_obj.is_absolute() {
            path_obj.to_path_buf()
        } else {
            cwd.join(path_obj)
        };
        // Check each component for symlinks
        let mut accumulated = std::path::PathBuf::new();
        for component in check_path.components() {
            accumulated.push(component);
            if matches!(std::fs::symlink_metadata(&accumulated), Ok(meta) if meta.file_type().is_symlink())
            {
                return Err(format!(
                    "Symlink blocked: '{}' is a symbolic link",
                    accumulated.display()
                ));
            }
        }
        // For reads: the file must already exist, so we can canonicalize directly.
        let canonical = dunce::canonicalize(path)
            .map_err(|e| format!("Path '{}' is invalid or does not exist: {}", path, e))?;
        if !canonical.starts_with(&cwd) {
            return Err(format!(
                "Path escape detected: '{}' resolves outside the working directory",
                path
            ));
        }
        Ok(canonical)
    }

    /// FINDING-05: Validate a filesystem path for write operations.
    /// For writes, the file may not yet exist — we validate the parent directory.
    /// Sprint 190: Symlink blocking — rejects any path containing a symbolic link.
    pub fn validate_fs_path_write(path: &str) -> Result<std::path::PathBuf, String> {
        let cwd = std::env::current_dir()
            .map_err(|e| format!("Cannot determine working directory: {}", e))?;
        let target = std::path::Path::new(path);
        // Resolve as an absolute path relative to cwd if not already absolute
        let abs = if target.is_absolute() {
            target.to_path_buf()
        } else {
            cwd.join(target)
        };
        // Sprint 190: Walk each component and reject symlinks
        let mut accumulated = std::path::PathBuf::new();
        for component in abs.components() {
            accumulated.push(component);
            if matches!(std::fs::symlink_metadata(&accumulated), Ok(meta) if meta.file_type().is_symlink())
            {
                return Err(format!(
                    "Symlink blocked: '{}' is a symbolic link",
                    accumulated.display()
                ));
            }
        }
        // Normalize by resolving ".." components without requiring the path to exist
        let mut normalized = std::path::PathBuf::new();
        for component in abs.components() {
            match component {
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                std::path::Component::CurDir => {}
                c => normalized.push(c),
            }
        }
        if !normalized.starts_with(&cwd) {
            return Err(format!(
                "Path escape detected: '{}' resolves outside the working directory",
                path
            ));
        }
        Ok(normalized)
    }

    fn resolve_ui_nodes(&mut self, nodes: Vec<Node>) -> Vec<Node> {
        let mut resolved = Vec::new();
        for node in nodes {
            match node {
                Node::UILabel(text) => {
                    let mut s = String::new();
                    if let ExecResult::Value(val) = self.evaluate(&text) {
                        if let RelType::Str(val_str) = val {
                            s = val_str;
                        } else {
                            s = format!("{}", val);
                        }
                    }
                    resolved.push(Node::UILabel(Box::new(Node::StringLiteral(s))));
                }
                Node::UIHBox(children) => {
                    resolved.push(Node::UIHBox(self.resolve_ui_nodes(children)));
                }
                Node::UIVBox(children) => {
                    resolved.push(Node::UIVBox(self.resolve_ui_nodes(children)));
                }
                Node::UIHorizontal(body) => {
                    resolved.push(Node::UIHorizontal(body));
                }
                _ => resolved.push(node),
            }
        }
        resolved
    }
}
