struct PointLight {
    pos:   vec4<f32>,   // xyz, pad
    color: vec4<f32>,   // rgb, intensity
}

struct MeshUniforms {
    view_proj: mat4x4<f32>,   // 64 bytes
    material:  vec4<f32>,     // RGBA            (bytes 64-79)
    pbr:       vec4<f32>,     // metallic, roughness, texture_id, normal_map_id (bytes 80-95)
    camera_pos: vec4<f32>,    // xyz, pad (bytes 96-111)
    lights:    array<PointLight, 4>, // (32 * 4 = 128 bytes)
}

@group(0) @binding(0)
var<uniform> u: MeshUniforms;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
@group(1) @binding(2)
var t_normal: texture_2d<f32>;

@group(2) @binding(0)
var<uniform> model_mat: mat4x4<f32>;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
}

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)       world_pos: vec3<f32>,
    @location(1)       normal:    vec3<f32>,
    @location(2)       uv:        vec2<f32>,
}

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    
    let world_pos4 = model_mat * vec4<f32>(v.position, 1.0);
    
    out.clip_pos  = u.view_proj * world_pos4;
    out.world_pos = world_pos4.xyz;
    
    // Use top-left 3x3 for normal transformation (ignoring scale for simplicity if uniform)
    let normal_mat = mat3x3<f32>(model_mat[0].xyz, model_mat[1].xyz, model_mat[2].xyz);
    out.normal    = normalize(normal_mat * v.normal);
    
    out.uv        = v.uv;
    
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let pbr = u.pbr;
    var albedo = u.material.rgb;
    
    // Sprint 158 EMERGENCY: Force high visibility
    let texture_id = pbr.z;
    if (texture_id > 0.5) {
        let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
        albedo = albedo * tex_color.rgb;
    } else {
        // If no texture, use a bright default based on position to see shapes
        albedo = vec3<f32>(0.8, 0.8, 0.9); 
    }
    
    // Return full brightness (unlit) to guarantee visibility
    return vec4<f32>(albedo, 1.0);
}
