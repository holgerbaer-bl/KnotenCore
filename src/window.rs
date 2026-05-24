use crate::natives::registry::{CachedMesh, RegistryWindowState, RenderCommand};
use std::collections::HashMap;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window as WinitWindow, WindowId};

pub struct KnotenApp {
    pub windows: HashMap<usize, RegistryWindowState>,
    pub window_id_map: HashMap<WindowId, usize>,
    pub compute_pipelines: HashMap<usize, wgpu::ComputePipeline>,
}

impl Default for KnotenApp {
    fn default() -> Self {
        Self::new()
    }
}

impl KnotenApp {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            window_id_map: HashMap::new(),
            compute_pipelines: HashMap::new(),
        }
    }

    fn handle_command(&mut self, event_loop: &ActiveEventLoop, cmd: RenderCommand) {
        match cmd {
            RenderCommand::CreateWindow {
                id,
                title,
                width,
                height,
                input,
            } => {
                let window_attributes = WinitWindow::default_attributes()
                    .with_title(title)
                    .with_inner_size(winit::dpi::PhysicalSize::new(width, height));

                let window = Arc::new(match event_loop.create_window(window_attributes) {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("[KnotenCore WGPU] Failed to create window: {}", e);
                        return;
                    }
                });
                let window_id = window.id();
                self.window_id_map.insert(window_id, id);

                // Initialize WGPU for this window
                let instance = wgpu::Instance::default();
                let surface = match instance.create_surface(window.clone()) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("[KnotenCore WGPU] Failed to create surface: {}", e);
                        return;
                    }
                };
                let adapter = match pollster::block_on(instance.request_adapter(
                    &wgpu::RequestAdapterOptions {
                        compatible_surface: Some(&surface),
                        ..Default::default()
                    },
                )) {
                    Some(a) => a,
                    None => {
                        eprintln!("[KnotenCore WGPU] Failed to find adapter");
                        return;
                    }
                };

                let (device, queue) = match pollster::block_on(
                    adapter.request_device(&wgpu::DeviceDescriptor::default(), None),
                ) {
                    Ok(dq) => dq,
                    Err(e) => {
                        eprintln!("[KnotenCore WGPU] Failed to create device: {}", e);
                        return;
                    }
                };
                let device = Arc::new(device);
                let queue = Arc::new(queue);

                let caps = surface.get_capabilities(&adapter);
                let config = wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: caps.formats[0],
                    width,
                    height,
                    present_mode: wgpu::PresentMode::Fifo,
                    alpha_mode: caps.alpha_modes[0],
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2,
                };
                surface.configure(&device, &config);

                // Setup basic 3D pipeline (placeholder / simplified from registry.rs)
                // In a real refactor, we'd move the pipeline setup code here.
                // For brevity, I'm assuming we'll use a shared initialization helper.

                let camera_bgl =
                    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("Mesh3D Camera/Uniform BGL"),
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::VERTEX
                                    | wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: false,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 2,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 3,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                },
                                count: None,
                            },
                        ],
                    });
                let material_bgl =
                    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("Material BGL"),
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
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
                    });

                let model_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Model BGL"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

                let pipeline_layout =
                    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Main Pipeline Layout"),
                        bind_group_layouts: &[&camera_bgl, &material_bgl, &model_bgl],
                        push_constant_ranges: &[],
                    });

                let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Mesh3D Blinn-Phong Shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("../assets/mesh3d.wgsl").into()),
                });

                let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("3D Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[
                            wgpu::VertexBufferLayout {
                                // RegistryVertex = [f32;3] position + [f32;3] normal + [f32;2] tex_coords = 32 bytes
                                array_stride: std::mem::size_of::<
                                    crate::natives::registry::RegistryVertex,
                                >()
                                    as wgpu::BufferAddress,
                                step_mode: wgpu::VertexStepMode::Vertex,
                                attributes: &[
                                    wgpu::VertexAttribute {
                                        offset: 0,
                                        shader_location: 0,
                                        format: wgpu::VertexFormat::Float32x3,
                                    }, // position
                                    wgpu::VertexAttribute {
                                        offset: 12,
                                        shader_location: 1,
                                        format: wgpu::VertexFormat::Float32x3,
                                    }, // normal
                                    wgpu::VertexAttribute {
                                        offset: 24,
                                        shader_location: 2,
                                        format: wgpu::VertexFormat::Float32x2,
                                    }, // uv
                                ],
                            },
                            wgpu::VertexBufferLayout {
                                array_stride: std::mem::size_of::<crate::executor::InstanceData>()
                                    as wgpu::BufferAddress,
                                step_mode: wgpu::VertexStepMode::Instance,
                                attributes: &[
                                    wgpu::VertexAttribute {
                                        offset: 0,
                                        shader_location: 3,
                                        format: wgpu::VertexFormat::Float32x4,
                                    },
                                    wgpu::VertexAttribute {
                                        offset: 16,
                                        shader_location: 4,
                                        format: wgpu::VertexFormat::Float32x4,
                                    },
                                    wgpu::VertexAttribute {
                                        offset: 32,
                                        shader_location: 5,
                                        format: wgpu::VertexFormat::Float32x4,
                                    },
                                    wgpu::VertexAttribute {
                                        offset: 48,
                                        shader_location: 6,
                                        format: wgpu::VertexFormat::Float32x4,
                                    },
                                    wgpu::VertexAttribute {
                                        offset: 64,
                                        shader_location: 7,
                                        format: wgpu::VertexFormat::Float32x4,
                                    },
                                    wgpu::VertexAttribute {
                                        offset: 80,
                                        shader_location: 8,
                                        format: wgpu::VertexFormat::Float32x4,
                                    },
                                ],
                            },
                        ],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: config.format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

                let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Depth Texture"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth32Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });
                let depth_texture_view =
                    depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

                // Sprint 85: create default 1x1 white texture first — needed by camera_bind_group entries 1/2/3
                let default_texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Default Texture"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &default_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &[255, 255, 255, 255],
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4),
                        rows_per_image: Some(1),
                    },
                    wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                );
                let default_view =
                    default_texture.create_view(&wgpu::TextureViewDescriptor::default());
                let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

                // Sprint 85: MeshUniforms: mat4(64) + vec4 material(16) + vec4 pbr(16) + vec4 camera_pos(16) + 4×PointLight(32×4=128) = 240 bytes
                let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Camera Buffer"),
                    size: 240,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                // Sprint 85: Satisfy all 4 bindings of camera_bgl (uniform + diffuse tex + sampler + normal tex)
                let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Camera Bind Group"),
                    layout: &camera_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: camera_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&default_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&default_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&default_view),
                        },
                    ],
                });

                let model_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Model Buffer"),
                    size: 64, // Mat4
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Model Bind Group"),
                    layout: &model_bgl,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: model_buffer.as_entire_binding(),
                    }],
                });

                // Default material bind group (material BGL has only binding 0/1 = texture/sampler)
                let default_texture_bind_group =
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Default Material Bind Group"),
                        layout: &material_bgl,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&default_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&default_sampler),
                            },
                        ],
                    });

                // We use the input directly provided by the registry instead of making a new one

                // Initialize Egui
                let egui_ctx = egui::Context::default();
                let viewport_id = egui::ViewportId::ROOT;
                let egui_state = egui_winit::State::new(
                    egui_ctx.clone(),
                    viewport_id,
                    &window,
                    Some(window.scale_factor() as f32),
                    None,
                    None,
                );
                let egui_renderer =
                    egui_wgpu::Renderer::new(&device, config.format, None, 1, false);

                self.windows.insert(
                    id,
                    RegistryWindowState {
                        window,
                        input,
                        surface,
                        surface_format: config.format,
                        config, // Sprint 86: store full config for resize handler
                        device,
                        queue,
                        pipeline,
                        width,
                        height,
                        clear_color: wgpu::Color::BLACK,
                        current_texture: None,
                        current_view: None,
                        encoder: None,
                        depth_texture_view,
                        camera_buffer,
                        camera_bind_group,
                        model_buffer,
                        model_bind_group,
                        geometry_cache: HashMap::new(),
                        texture_cache: HashMap::new(),
                        default_texture_bind_group,
                        commands: Vec::new(),
                        scene_graph: HashMap::new(),
                        lights: HashMap::new(),
                        egui_ctx,
                        egui_state,
                        egui_renderer,
                        ui_tree: Vec::new(),
                    },
                );
                // Sprint 174: Trigger initial frame — subsequent frames are
                // requested at the end of each RedrawRequested handler.
                if let Some(state) = self.windows.get(&id) {
                    state.window.request_redraw();
                }
            }
            RenderCommand::UpdateWindow(id) => {
                if let Some(state) = self.windows.get_mut(&id) {
                    state.window.request_redraw();
                }
            }
            RenderCommand::UpdateUI { window_id, nodes } => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    state.ui_tree = nodes;
                    state.window.request_redraw();
                }
            }
            RenderCommand::CloseWindow(id) => {
                self.windows.remove(&id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            RenderCommand::LoadComputeShader { id, source } => {
                if let Some(state) = self.windows.values().next() {
                    use std::collections::hash_map::Entry;
                    if let Entry::Vacant(e) = self.compute_pipelines.entry(id) {
                        let shader =
                            state
                                .device
                                .create_shader_module(wgpu::ShaderModuleDescriptor {
                                    label: Some("Compute Shader"),
                                    source: wgpu::ShaderSource::Wgsl(source.into()),
                                });
                        let pipeline = state.device.create_compute_pipeline(
                            &wgpu::ComputePipelineDescriptor {
                                label: Some("Compute Pipeline"),
                                layout: None,
                                module: &shader,
                                entry_point: Some("main"),
                                compilation_options: Default::default(),
                                cache: None,
                            },
                        );
                        e.insert(pipeline);
                    }
                } else {
                    eprintln!("LoadComputeShader failed: No WGPU window/device available.");
                }
            }
            RenderCommand::DispatchCompute {
                shader_id,
                x,
                y,
                z,
                inputs,
            } => {
                if let Some(state) = self.windows.values().next() {
                    if let Some(pipeline) = self.compute_pipelines.get(&shader_id) {
                        // Sprint 187: Serialize inputs into a storage buffer
                        let (data_bytes, element_count) = inputs_to_storage_buffer(&inputs);

                        if element_count > 0 {
                            let storage = state.device.create_buffer(&wgpu::BufferDescriptor {
                                label: Some("Compute Storage Buffer"),
                                size: data_bytes.len() as u64,
                                usage: wgpu::BufferUsages::STORAGE
                                    | wgpu::BufferUsages::COPY_DST,
                                mapped_at_creation: false,
                            });
                            state.queue.write_buffer(&storage, 0, &data_bytes);

                            let bind_group_layout = pipeline.get_bind_group_layout(0);
                            let bind_group =
                                state
                                    .device
                                    .create_bind_group(&wgpu::BindGroupDescriptor {
                                        label: Some("Compute Bind Group"),
                                        layout: &bind_group_layout,
                                        entries: &[wgpu::BindGroupEntry {
                                            binding: 0,
                                            resource: storage.as_entire_binding(),
                                        }],
                                    });

                            let mut encoder = state.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor {
                                    label: Some("Compute Encoder"),
                                },
                            );
                            {
                                let mut cpass = encoder.begin_compute_pass(
                                    &wgpu::ComputePassDescriptor {
                                        label: Some("Compute Pass"),
                                        timestamp_writes: None,
                                    },
                                );
                                cpass.set_pipeline(pipeline);
                                cpass.set_bind_group(0, &bind_group, &[]);
                                cpass.dispatch_workgroups(x, y, z);
                            }
                            state.queue.submit(std::iter::once(encoder.finish()));
                        } else {
                            // No data inputs — dispatch without bind group (backward compat)
                            let mut encoder = state.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor {
                                    label: Some("Compute Encoder"),
                                },
                            );
                            {
                                let mut cpass = encoder.begin_compute_pass(
                                    &wgpu::ComputePassDescriptor {
                                        label: Some("Compute Pass"),
                                        timestamp_writes: None,
                                    },
                                );
                                cpass.set_pipeline(pipeline);
                                cpass.dispatch_workgroups(x, y, z);
                            }
                            state.queue.submit(std::iter::once(encoder.finish()));
                        }
                    } else {
                        eprintln!("DispatchCompute failed: Shader {} not found.", shader_id);
                    }
                } else {
                    eprintln!("DispatchCompute failed: No WGPU window/device available.");
                }
            }
            RenderCommand::AddMesh {
                name,
                vertices,
                indices,
            } => {
                for state in self.windows.values_mut() {
                    use wgpu::util::DeviceExt;
                    let vertex_buffer =
                        state
                            .device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some(&format!("Mesh {} VBO", name)),
                                contents: bytemuck::cast_slice(&vertices),
                                usage: wgpu::BufferUsages::VERTEX,
                            });
                    let index_buffer =
                        state
                            .device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some(&format!("Mesh {} IBO", name)),
                                contents: bytemuck::cast_slice(&indices),
                                usage: wgpu::BufferUsages::INDEX,
                            });
                    state.geometry_cache.insert(
                        name.clone(),
                        CachedMesh {
                            vertex_buffer,
                            index_buffer,
                            index_count: indices.len() as u32,
                        },
                    );
                }
            }
            RenderCommand::LoadTexture {
                id,
                width,
                height,
                rgba,
            } => {
                for state in self.windows.values_mut() {
                    let texture_size = wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    };
                    let diffuse_texture = state.device.create_texture(&wgpu::TextureDescriptor {
                        size: texture_size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        label: Some(&format!("Texture {}", id)),
                        view_formats: &[],
                    });

                    state.queue.write_texture(
                        wgpu::ImageCopyTexture {
                            texture: &diffuse_texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &rgba,
                        wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * width),
                            rows_per_image: Some(height),
                        },
                        texture_size,
                    );

                    let diffuse_texture_view =
                        diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let diffuse_sampler = state.device.create_sampler(&wgpu::SamplerDescriptor {
                        address_mode_u: wgpu::AddressMode::Repeat,
                        address_mode_v: wgpu::AddressMode::Repeat,
                        address_mode_w: wgpu::AddressMode::Repeat,
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Linear,
                        mipmap_filter: wgpu::FilterMode::Linear,
                        ..Default::default()
                    });

                    let material_bgl = state.pipeline.get_bind_group_layout(1);
                    let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &material_bgl,
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
                        label: Some(&format!("Texture Bind Group {}", id)),
                    });

                    state.texture_cache.insert(id, bind_group);
                }
            }
            RenderCommand::ExitEventLoop => {
                event_loop.exit();
            }
            // Sprint 86: SetCamera — write the view-proj matrix to the per-window camera UBO
            RenderCommand::SetCamera {
                window_id,
                view_proj,
            } => {
                // If window_id == 0, broadcast to all windows (legacy 4-arg call)
                if window_id == 0 {
                    for state in self.windows.values_mut() {
                        state.queue.write_buffer(
                            &state.camera_buffer,
                            0,
                            bytemuck::cast_slice(view_proj.as_flattened()),
                        );
                        let mut input = state.input.lock().unwrap_or_else(|e| e.into_inner());
                        input.view_proj = view_proj;
                    }
                } else if let Some(state) = self.windows.get_mut(&window_id) {
                    state.queue.write_buffer(
                        &state.camera_buffer,
                        0,
                        bytemuck::cast_slice(view_proj.as_flattened()),
                    );
                    let mut input = state.input.lock().unwrap_or_else(|e| e.into_inner());
                    input.view_proj = view_proj;
                }
            }
            RenderCommand::SpawnEntity {
                window_id,
                entity_id,
                mesh_name,
                texture_id,
                transform,
            } => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    state.scene_graph.insert(
                        entity_id,
                        crate::natives::registry::SceneEntity {
                            mesh_name,
                            texture_id,
                            transform,
                        },
                    );
                }
            }
            RenderCommand::RemoveEntity {
                window_id,
                entity_id,
            } => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    state.scene_graph.remove(&entity_id);
                }
            }
            RenderCommand::UpdateEntityTransform {
                window_id,
                entity_id,
                transform,
            } => {
                if let Some(state) = self.windows.get_mut(&window_id)
                    && let Some(entity) = state.scene_graph.get_mut(&entity_id)
                {
                    entity.transform = transform;
                }
            }
            // Sprint 167: Spawn a dynamic point light
            RenderCommand::SpawnLight {
                window_id,
                light_id,
                x,
                y,
                z,
                r,
                g,
                b,
                intensity,
            } => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    state.lights.insert(
                        light_id,
                        crate::natives::registry::SceneLight {
                            position: [x, y, z],
                            color: [r, g, b],
                            intensity,
                        },
                    );
                }
            }
            // Sprint 167: Update position of an existing point light
            RenderCommand::UpdateLightPosition {
                window_id,
                light_id,
                x,
                y,
                z,
            } => {
                if let Some(state) = self.windows.get_mut(&window_id)
                    && let Some(light) = state.lights.get_mut(&light_id)
                {
                    light.position = [x, y, z];
                }
            }
        }
    }
}

impl ApplicationHandler<RenderCommand> for KnotenApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Sprint 174: ControlFlow::Wait eliminates 100% CPU idle usage.
        // Frames are driven by request_redraw() at the end of each RedrawRequested.
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, cmd: RenderCommand) {
        self.handle_command(event_loop, cmd);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let registry_id = match self.window_id_map.get(&window_id) {
            Some(&id) => id,
            None => return,
        };

        let state = match self.windows.get_mut(&registry_id) {
            Some(s) => s,
            None => return,
        };

        // Feed egui
        let egui_response = state.egui_state.on_window_event(&state.window, &event);
        if egui_response.consumed {
            // Egui consumed this event, you might want to early-return for certain events
            // if you don't want 3D logic to also process them. For now, we continue.
        }

        match event {
            WindowEvent::CloseRequested => {
                self.windows.remove(&registry_id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::KeyboardInput { event: key_ev, .. } => {
                let mut input = state.input.lock().unwrap_or_else(|e| e.into_inner());
                if let winit::keyboard::PhysicalKey::Code(code) = key_ev.physical_key {
                    if key_ev.state == winit::event::ElementState::Pressed {
                        input.keys.insert(code);
                    } else {
                        input.keys.remove(&code);
                    }

                    // Sprint 109: Lock-free global input array
                    let key_idx = match code {
                        winit::keyboard::KeyCode::KeyW => Some(1),
                        winit::keyboard::KeyCode::KeyA => Some(2),
                        winit::keyboard::KeyCode::KeyS => Some(3),
                        winit::keyboard::KeyCode::KeyD => Some(4),
                        winit::keyboard::KeyCode::Space => Some(5),
                        winit::keyboard::KeyCode::ArrowUp => Some(6),
                        winit::keyboard::KeyCode::ArrowDown => Some(7),
                        winit::keyboard::KeyCode::ArrowLeft => Some(8),
                        winit::keyboard::KeyCode::ArrowRight => Some(9),
                        _ => None,
                    };

                    if let Some(idx) = key_idx {
                        crate::natives::registry::GLOBAL_KEYS[idx].store(
                            key_ev.state == winit::event::ElementState::Pressed,
                            std::sync::atomic::Ordering::Relaxed,
                        );
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let mut input = state.input.lock().unwrap_or_else(|e| e.into_inner());
                input.mouse_x = position.x as f32;
                input.mouse_y = position.y as f32;
            }
            WindowEvent::MouseInput {
                state: element_state,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                let mut input = state.input.lock().unwrap_or_else(|e| e.into_inner());
                input.mouse_left_down = element_state == winit::event::ElementState::Pressed;
                if element_state == winit::event::ElementState::Pressed {
                    input.mouse_clicked = true;
                }
            }
            WindowEvent::Resized(physical_size)
                if physical_size.width > 0 && physical_size.height > 0 =>
            {
                state.width = physical_size.width;
                state.height = physical_size.height;
                {
                    let mut input = state.input.lock().unwrap_or_else(|e| e.into_inner());
                    input.window_width = physical_size.width as f32;
                    input.window_height = physical_size.height as f32;
                }
                // Sprint 86 FIX: mutate stored config and reconfigure — no hardcoded format
                state.config.width = physical_size.width;
                state.config.height = physical_size.height;
                state.surface.configure(&state.device, &state.config);

                let depth_texture = state.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Depth Texture"),
                    size: wgpu::Extent3d {
                        width: state.width,
                        height: state.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth32Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });
                state.depth_texture_view =
                    depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
            }
            WindowEvent::RedrawRequested => {
                // Sprint 86: write a default view-proj only if no SetCamera has been received yet
                // (camera_buffer starts zero, so we provide a sane fallback each frame).
                let aspect = state.width as f32 / state.height.max(1) as f32;
                let proj = glam::Mat4::perspective_rh(60_f32.to_radians(), aspect, 0.1, 1000.0);
                let view = glam::Mat4::look_at_rh(
                    glam::Vec3::new(0.0, 2.0, 5.0),
                    glam::Vec3::ZERO,
                    glam::Vec3::Y,
                );
                let view_proj = proj * view;
                // Write the 64-byte view_proj into the camera UBO at offset 0.
                // If the script already called registry_set_camera_for_window, it overwrites this.
                state.queue.write_buffer(
                    &state.camera_buffer,
                    0,
                    bytemuck::cast_slice(&view_proj.to_cols_array()),
                );

                // Sprint 167: Write camera_pos into the UBO at offset 96 (after mat4 + material + pbr)
                let cam_pos = view.inverse().w_axis;
                let camera_pos_data: [f32; 4] = [cam_pos.x, cam_pos.y, cam_pos.z, 0.0];
                state.queue.write_buffer(
                    &state.camera_buffer,
                    96,
                    bytemuck::cast_slice(&camera_pos_data),
                );

                // Sprint 167: Write light data into the UBO at offset 112 (4 × PointLight @ 32 bytes each)
                // PointLight layout: vec4 pos (xyz, pad) + vec4 color (rgb, intensity)
                let mut light_ubo_data = [0.0_f32; 32]; // 4 lights × 8 floats
                for (i, light) in state.lights.values().enumerate() {
                    if i >= 4 {
                        break;
                    }
                    let base = i * 8;
                    light_ubo_data[base] = light.position[0];
                    light_ubo_data[base + 1] = light.position[1];
                    light_ubo_data[base + 2] = light.position[2];
                    light_ubo_data[base + 3] = 0.0; // pad
                    light_ubo_data[base + 4] = light.color[0];
                    light_ubo_data[base + 5] = light.color[1];
                    light_ubo_data[base + 6] = light.color[2];
                    light_ubo_data[base + 7] = light.intensity;
                }
                state.queue.write_buffer(
                    &state.camera_buffer,
                    112,
                    bytemuck::cast_slice(&light_ubo_data),
                );

                {
                    let mut input = state.input.lock().unwrap_or_else(|e| e.into_inner());
                    input.view_proj = view_proj.to_cols_array_2d();
                }

                // Drain and process all pending RenderCommands for this window
                let output = match state.surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(wgpu::SurfaceError::Outdated) | Err(wgpu::SurfaceError::Lost) => {
                        state.surface.configure(&state.device, &state.config);
                        return;
                    }
                    Err(wgpu::SurfaceError::Timeout) => return,
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        eprintln!(
                            "[KnotenCore WGPU] Out of memory when acquiring surface — skipping frame"
                        );
                        return;
                    }
                };
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = state
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                let raw_input = state.egui_state.take_egui_input(&state.window);
                state.egui_ctx.begin_pass(raw_input);

                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(egui::Color32::from_black_alpha(0)))
                    .show(&state.egui_ctx, |ui| {
                        for node in &state.ui_tree {
                            render_egui_node(ui, node);
                        }
                    });

                let egui::FullOutput {
                    platform_output,
                    textures_delta,
                    shapes,
                    pixels_per_point,
                    ..
                } = state.egui_ctx.end_pass();

                state
                    .egui_state
                    .handle_platform_output(&state.window, platform_output);
                let paint_jobs = state.egui_ctx.tessellate(shapes, pixels_per_point);

                // Update Egui Textures & Buffers
                for (id, image_delta) in &textures_delta.set {
                    state.egui_renderer.update_texture(
                        &state.device,
                        &state.queue,
                        *id,
                        image_delta,
                    );
                }

                let screen_desc = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [state.config.width, state.config.height],
                    pixels_per_point,
                };

                state.egui_renderer.update_buffers(
                    &state.device,
                    &state.queue,
                    &mut encoder,
                    &paint_jobs,
                    &screen_desc,
                );

                // Drain commands for this frame
                state.commands.clear();

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(state.clear_color),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &state.depth_texture_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    rpass.set_pipeline(&state.pipeline);
                    rpass.set_bind_group(0, &state.camera_bind_group, &[]);

                    for entity in state.scene_graph.values() {
                        let mesh_name = &entity.mesh_name;
                        let texture_id = entity.texture_id;
                        let transform = entity.transform;

                        if let Some(mesh) = state.geometry_cache.get(mesh_name) {
                            // Update model matrix
                            state.queue.write_buffer(
                                &state.model_buffer,
                                0,
                                bytemuck::cast_slice(&transform.to_cols_array()),
                            );

                            rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                            rpass.set_index_buffer(
                                mesh.index_buffer.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );

                            let mat_bg = state
                                .texture_cache
                                .get(&texture_id)
                                .unwrap_or(&state.default_texture_bind_group);
                            rpass.set_bind_group(1, mat_bg, &[]);
                            rpass.set_bind_group(2, &state.model_bind_group, &[]);

                            rpass.draw_indexed(0..mesh.index_count, 0, 0..1);
                        }
                    }
                }

                // Render Egui directly onto the screen (load, don't clear)
                let mut egui_pass = encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Egui UI Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    })
                    .forget_lifetime();
                state
                    .egui_renderer
                    .render(&mut egui_pass, &paint_jobs, &screen_desc);
                drop(egui_pass);

                for id in textures_delta.free {
                    state.egui_renderer.free_texture(&id);
                }

                state.queue.submit(Some(encoder.finish()));
                output.present();

                // Sprint 174: Request next frame — paced by WGPU FIFO VSync.
                // ControlFlow::Wait lets the thread sleep between frames.
                state.window.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // Sprint 174: No longer auto-requests redraws in idle.
        // Redraws are now driven by request_redraw() at the end of each
        // RedrawRequested handler — the WGPU FIFO present already paces
        // frames to VSync, and ControlFlow::Wait eliminates CPU spin.
    }
}

/// Sprint 162: Render a single UI AST node into an egui Ui context.
///
/// Event routing:
///   - UIButton click  → `ui_button_signal(label)` sets a flag in UI_BUTTON_EVENTS
///   - UITextInput edit → `ui_text_write(key, val)` persists into UI_TEXT_BUFFERS
///
/// The VM reads these stores via `registry_ui_poll_button` / `registry_ui_read_text`.
fn render_egui_node(ui: &mut egui::Ui, node: &crate::ast::Node) {
    match node {
        // ── UIButton ─────────────────────────────────────────────
        // Label is the button text and its event-routing key.
        crate::ast::Node::UIButton(text_node) => {
            let label = if let crate::ast::Node::StringLiteral(s) = &**text_node {
                s.clone()
            } else {
                "Button".to_string()
            };
            if ui.button(&label).clicked() {
                crate::natives::ui::ui_button_signal(&label);
            }
        }

        // ── UITextInput ───────────────────────────────────────────
        // The seed value (StringLiteral) is also used as the buffer key,
        // enabling multiple independent text inputs in one window.
        crate::ast::Node::UITextInput(seed_node) => {
            let key = if let crate::ast::Node::StringLiteral(s) = &**seed_node {
                s.clone()
            } else {
                // Fall back to the legacy single buffer
                if let Ok(mut buf) = crate::natives::ui::UI_TEXT_INPUT_BUFFER.lock() {
                    ui.text_edit_singleline(&mut *buf);
                }
                return;
            };
            // Read current value from the keyed buffer, edit in-place, write back.
            let mut val = crate::natives::ui::ui_text_read(&key);
            ui.text_edit_singleline(&mut val);
            crate::natives::ui::ui_text_write(&key, val);
        }

        // ── UILabel ───────────────────────────────────────────────
        crate::ast::Node::UILabel(text_node) => {
            if let crate::ast::Node::StringLiteral(s) = &**text_node {
                ui.label(s);
            }
        }

        // ── UIHBox / UIVBox ───────────────────────────────────────
        crate::ast::Node::UIHBox(children) => {
            ui.horizontal(|ui| {
                for child in children {
                    render_egui_node(ui, child);
                }
            });
        }
        crate::ast::Node::UIVBox(children) => {
            ui.vertical(|ui| {
                for child in children {
                    render_egui_node(ui, child);
                }
            });
        }

        // Unsupported nodes are silently ignored (no panic).
        _ => {}
    }
}

/// Sprint 187: Serialize RelType inputs into a packed f32 byte buffer for GPU storage.
/// Returns (byte_data, element_count). Floats are written in little-endian f32 format.
/// Structured objects and arrays are flattened.
fn inputs_to_storage_buffer(inputs: &[crate::executor::RelType]) -> (Vec<u8>, u32) {
    let mut floats: Vec<f32> = Vec::new();
    for input in inputs {
        collect_floats(input, &mut floats);
    }
    if floats.is_empty() {
        return (Vec::new(), 0);
    }
    let bytes: Vec<u8> = floats
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();
    (bytes, floats.len() as u32)
}

fn collect_floats(rel: &crate::executor::RelType, out: &mut Vec<f32>) {
    match rel {
        crate::executor::RelType::Float(f) => out.push(*f as f32),
        crate::executor::RelType::Int(i) => out.push(*i as f32),
        crate::executor::RelType::Array(arr) => {
            for item in arr {
                collect_floats(item, out);
            }
        }
        crate::executor::RelType::Object(map) => {
            for v in map.values() {
                collect_floats(v, out);
            }
        }
        _ => {} // Skip non-numeric types
    }
}
