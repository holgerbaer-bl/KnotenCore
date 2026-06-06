// Sprint 239: Particle render shader — reads compute output buffers directly
// @group(0) @binding(0): particle positions (read-only storage, from compute pass)
// @group(0) @binding(2): camera view-projection uniform

struct CameraUniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<storage, read> positions: array<vec3<f32>>;
@group(0) @binding(2) var<uniform> camera: CameraUniforms;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var out: VertexOutput;
    out.pos = camera.view_proj * vec4(positions[idx], 1.0);
    out.color = vec4(1.0, 0.5, 0.2, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
