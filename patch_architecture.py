import os
import re

def patch_registry():
    with open('src/natives/registry.rs', 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 1. Update RenderCommand enum
    content = re.sub(
        r'DrawSphere \{[^\}]+\},',
        '',
        content
    )
    content = re.sub(
        r'DrawCube \{[^\}]+\},',
        '',
        content
    )
    content = re.sub(
        r'DrawCylinder \{[^\}]+\},',
        '',
        content
    )
    content = re.sub(
        r'DrawQuad3D \{[^\}]+\},',
        '',
        content
    )
    
    # Insert new commands after CreateWindow
    content = content.replace(
        'UpdateWindow(usize),',
        '''SpawnEntity {
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
    UpdateWindow(usize),'''
    )
    
    # 2. Add new entity FFI functions
    # Replace old draw functions
    new_ffi = """
static NEXT_ENTITY_ID: Mutex<usize> = Mutex::new(1);

pub fn registry_spawn_cube(window_id: usize, texture_id: usize, w: f32, h: f32, d: f32, x: f32, y: f32, z: f32) -> i64 {
    let mut id_guard = NEXT_ENTITY_ID.lock().unwrap();
    let entity_id = *id_guard;
    *id_guard += 1;
    let t = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
    let s = glam::Mat4::from_scale(glam::Vec3::new(w, h, d));
    send_render_command(RenderCommand::SpawnEntity {
        window_id,
        entity_id,
        mesh_name: "cube".to_string(),
        texture_id,
        transform: t * s,
    });
    entity_id as i64
}

pub fn registry_spawn_sphere(window_id: usize, texture_id: usize, r: f32, rings: i32, sectors: i32, x: f32, y: f32, z: f32) -> i64 {
    let mut id_guard = NEXT_ENTITY_ID.lock().unwrap();
    let entity_id = *id_guard;
    *id_guard += 1;
    let t = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
    let s = glam::Mat4::from_scale(glam::Vec3::splat(r));
    send_render_command(RenderCommand::SpawnEntity {
        window_id,
        entity_id,
        mesh_name: format!("sphere_{}_{}", rings, sectors),
        texture_id,
        transform: t * s,
    });
    entity_id as i64
}

pub fn registry_update_entity_transform(window_id: usize, entity_id: usize, x: f32, y: f32, z: f32) {
    let t = glam::Mat4::from_translation(glam::Vec3::new(x, y, z));
    send_render_command(RenderCommand::UpdateEntityTransform {
        window_id,
        entity_id,
        transform: t,
    });
}
"""
    # Remove old registry_draw_*
    content = re.sub(r'pub fn registry_draw_cube.*?\}\n', '', content, flags=re.DOTALL)
    content = re.sub(r'pub fn registry_draw_sphere.*?\}\n', '', content, flags=re.DOTALL)
    content = re.sub(r'pub fn registry_draw_cylinder.*?\}\n', '', content, flags=re.DOTALL)
    content = re.sub(r'pub fn registry_draw_quad_3d.*?\}\n', '', content, flags=re.DOTALL)
    
    with open('src/natives/registry.rs', 'w', encoding='utf-8') as f:
        f.write(content + "\n" + new_ffi)

def patch_window():
    with open('src/window.rs', 'r', encoding='utf-8') as f:
        content = f.read()

    # 1. Add SceneGraph to RegistryWindowState
    content = content.replace(
        'pub commands: Vec<RenderCommand>,',
        '''pub commands: Vec<RenderCommand>,
    pub scene_graph: HashMap<usize, SceneEntity>,'''
    )
    
    content = content.replace(
        'pub struct RegistryWindowState',
        '''pub struct SceneEntity {
    pub mesh_name: String,
    pub texture_id: usize,
    pub transform: glam::Mat4,
}

pub struct RegistryWindowState'''
    )
    
    content = content.replace(
        'commands: Vec::new(),',
        'commands: Vec::new(),\n                        scene_graph: HashMap::new(),'
    )
    
    # 3. Handle commands in handle_command
    handle_cmd_add = '''
            RenderCommand::SpawnEntity { window_id, entity_id, mesh_name, texture_id, transform } => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    state.scene_graph.insert(entity_id, SceneEntity { mesh_name, texture_id, transform });
                }
            }
            RenderCommand::UpdateEntityTransform { window_id, entity_id, transform } => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    if let Some(entity) = state.scene_graph.get_mut(&entity_id) {
                        entity.transform = transform;
                    }
                }
            }
    '''
    content = content.replace(
        'RenderCommand::ExitEventLoop => {',
        handle_cmd_add + '            RenderCommand::ExitEventLoop => {'
    )
    
    # Remove old match arm for draw commands
    content = re.sub(
        r'draw_cmd => \{\s*// Determine target window id.*?\}\n\s*\}',
        '_ => {}',
        content,
        flags=re.DOTALL
    )
    
    # 4. In RedrawRequested, render the scene graph
    # Find `for cmd in frame_cmds {` block
    render_logic = '''
                    for (_, entity) in &state.scene_graph {
                        if let Some(mesh) = state.geometry_cache.get(&entity.mesh_name) {
                            state.queue.write_buffer(
                                &state.model_buffer,
                                0,
                                bytemuck::cast_slice(&entity.transform.to_cols_array()),
                            );
                            rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                            rpass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                            
                            let mut has_texture = false;
                            if entity.texture_id > 0 {
                                if let Some(bg) = state.texture_cache.get(&entity.texture_id) {
                                    rpass.set_bind_group(1, bg, &[]);
                                    has_texture = true;
                                }
                            }
                            if !has_texture {
                                rpass.set_bind_group(1, &state.default_texture_bind_group, &[]);
                            }
                            rpass.draw_indexed(0..mesh.index_count, 0, 0..1);
                        }
                    }
    '''
    content = re.sub(
        r'for cmd in frame_cmds \{.*?rpass\.draw_indexed\(0\.\.mesh\.index_count, 0, 0\.\.1\);\s*\}\s*\}',
        render_logic,
        content,
        flags=re.DOTALL
    )
    
    with open('src/window.rs', 'w', encoding='utf-8') as f:
        f.write(content)

def patch_bridge():
    with open('src/natives/bridge.rs', 'r', encoding='utf-8') as f:
        content = f.read()

    new_bridge_ffi = '''
                "registry_spawn_cube" => {
                    if args.len() == 8 {
                        let get_float = |arg: &RelType| -> Result<f32, String> {
                            match arg {
                                RelType::Float(f) => Ok(*f as f32),
                                RelType::Int(i) => Ok(*i as f32),
                                _ => Err("Expected Float or Int".to_string()),
                            }
                        };
                        let win_handle = match &args[0] {
                            RelType::Handle(crate::executor::NativeHandle(h)) => Some(*h),
                            RelType::Int(i) => Some(*i as usize),
                            _ => None,
                        };
                        let tex_handle = match &args[1] {
                            RelType::Handle(crate::executor::NativeHandle(h)) => Some(*h),
                            RelType::Int(i) => Some(*i as usize),
                            _ => None,
                        };

                        if let (Some(win), Some(tex)) = (win_handle, tex_handle) {
                            let w = get_float(&args[2]);
                            let h = get_float(&args[3]);
                            let d = get_float(&args[4]);
                            let x = get_float(&args[5]);
                            let y = get_float(&args[6]);
                            let z = get_float(&args[7]);
                            
                            if let (Ok(w), Ok(h), Ok(d), Ok(x), Ok(y), Ok(z)) = (w, h, d, x, y, z) {
                                let id = crate::natives::registry::registry_spawn_cube(
                                    win, tex, w, h, d, x, y, z,
                                );
                                return Some(ExecResult::Value(RelType::Int(id)));
                            } else {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] registry_spawn_cube type error: arguments must be numeric".to_string(),
                                    node: "Native::Bridge::registry_spawn_cube".into()
                                });
                            }
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_spawn_cube expects (win, tex, w, h, d, x, y, z)".to_string(),
                        node: "Native::Bridge::registry_spawn_cube".into()
                    })
                }
                "registry_spawn_sphere" => {
                    if args.len() == 8 {
                        let get_float = |arg: &RelType| -> Result<f32, String> {
                            match arg {
                                RelType::Float(f) => Ok(*f as f32),
                                RelType::Int(i) => Ok(*i as f32),
                                _ => Err("Expected Float or Int".to_string()),
                            }
                        };
                        let win_handle = match &args[0] {
                            RelType::Handle(crate::executor::NativeHandle(h)) => Some(*h),
                            RelType::Int(i) => Some(*i as usize),
                            _ => None,
                        };
                        let tex_handle = match &args[1] {
                            RelType::Handle(crate::executor::NativeHandle(h)) => Some(*h),
                            RelType::Int(i) => Some(*i as usize),
                            _ => None,
                        };

                        if let (Some(win), Some(tex)) = (win_handle, tex_handle) {
                            let r = get_float(&args[2]);
                            let rings = get_float(&args[3]);
                            let sectors = get_float(&args[4]);
                            let x = get_float(&args[5]);
                            let y = get_float(&args[6]);
                            let z = get_float(&args[7]);
                            
                            if let (Ok(r), Ok(rings), Ok(sectors), Ok(x), Ok(y), Ok(z)) = (r, rings, sectors, x, y, z) {
                                let id = crate::natives::registry::registry_spawn_sphere(
                                    win, tex, r, rings as i32, sectors as i32, x, y, z,
                                );
                                return Some(ExecResult::Value(RelType::Int(id)));
                            } else {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] registry_spawn_sphere type error: arguments must be numeric".to_string(),
                                    node: "Native::Bridge::registry_spawn_sphere".into()
                                });
                            }
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_spawn_sphere expects (win, tex, r, rings, sectors, x, y, z)".to_string(),
                        node: "Native::Bridge::registry_spawn_sphere".into()
                    })
                }
                "registry_update_entity_transform" => {
                    if args.len() == 5 {
                        let get_float = |arg: &RelType| -> Result<f32, String> {
                            match arg {
                                RelType::Float(f) => Ok(*f as f32),
                                RelType::Int(i) => Ok(*i as f32),
                                _ => Err("Expected Float or Int".to_string()),
                            }
                        };
                        let win_handle = match &args[0] {
                            RelType::Handle(crate::executor::NativeHandle(h)) => Some(*h),
                            RelType::Int(i) => Some(*i as usize),
                            _ => None,
                        };
                        let entity_handle = match &args[1] {
                            RelType::Handle(crate::executor::NativeHandle(h)) => Some(*h),
                            RelType::Int(i) => Some(*i as usize),
                            _ => None,
                        };

                        if let (Some(win), Some(ent)) = (win_handle, entity_handle) {
                            let x = get_float(&args[2]);
                            let y = get_float(&args[3]);
                            let z = get_float(&args[4]);
                            
                            if let (Ok(x), Ok(y), Ok(z)) = (x, y, z) {
                                crate::natives::registry::registry_update_entity_transform(win, ent, x, y, z);
                                return Some(ExecResult::Value(RelType::Void));
                            } else {
                                return Some(ExecResult::Fault {
                                    msg: "[FFI] registry_update_entity_transform type error: coordinates must be numeric".to_string(),
                                    node: "Native::Bridge::registry_update_entity_transform".into()
                                });
                            }
                        }
                    }
                    Some(ExecResult::Fault {
                        msg: "[FFI] registry_update_entity_transform expects (win, entity, x, y, z)".to_string(),
                        node: "Native::Bridge::registry_update_entity_transform".into()
                    })
                }
'''
    
    content = re.sub(r'"registry_draw_cube" => \{.*?\n                \}\n', '', content, flags=re.DOTALL)
    content = re.sub(r'"registry_draw_sphere" => \{.*?\n                \}\n', '', content, flags=re.DOTALL)
    content = re.sub(r'"registry_draw_cylinder" => \{.*?\n                \}\n', '', content, flags=re.DOTALL)
    content = re.sub(r'"registry_draw_quad_3d" => \{.*?\n                \}\n', '', content, flags=re.DOTALL)
    content = re.sub(r'"registry_draw_entity" => \{.*?\n                \}\n', '', content, flags=re.DOTALL)
    
    content = content.replace(
        '"registry_set_camera" => {',
        new_bridge_ffi + '                "registry_set_camera" => {'
    )
    
    with open('src/natives/bridge.rs', 'w', encoding='utf-8') as f:
        f.write(content)

patch_registry()
patch_window()
patch_bridge()
print("Patching complete.")
