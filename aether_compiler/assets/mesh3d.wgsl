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

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
}

// Sprint 206: Instance data — per-entity transform matrix uploaded as vertex buffer 1.
// Matches InstanceData struct layout: transform (64B) + color_offset (16B) + pbr (16B) = 96B
struct InstanceIn {
    @location(3) model_col0: vec4<f32>,
    @location(4) model_col1: vec4<f32>,
    @location(5) model_col2: vec4<f32>,
    @location(6) model_col3: vec4<f32>,
    @location(7) color_offset: vec4<f32>,
    @location(8) material_pbr: vec4<f32>,
}

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)   world_pos: vec3<f32>,
    @location(1)   normal:    vec3<f32>,
    @location(2)   uv:        vec2<f32>,
}

@vertex
fn vs_main(v: VertexIn, instance: InstanceIn) -> VertexOut {
    var out: VertexOut;

    let model_mat = mat4x4<f32>(
        instance.model_col0,
        instance.model_col1,
        instance.model_col2,
        instance.model_col3,
    );

    let world_pos4 = model_mat * vec4<f32>(v.position, 1.0);

    out.clip_pos  = u.view_proj * world_pos4;
    out.world_pos = world_pos4.xyz;

    let normal_mat = mat3x3<f32>(model_mat[0].xyz, model_mat[1].xyz, model_mat[2].xyz);
    out.normal    = normalize(normal_mat * v.normal);

    out.uv        = v.uv;

    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let pbr = u.pbr;
    var albedo = u.material.rgb;

    // Texture sampling
    let texture_id = pbr.z;
    if (texture_id > 0.5) {
        let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
        albedo = albedo * tex_color.rgb;
    } else {
        albedo = vec3<f32>(0.8, 0.8, 0.9);
    }

    // Sprint 167: Blinn-Phong Lighting
    let N = normalize(in.normal);
    let V = normalize(u.camera_pos.xyz - in.world_pos);

    let ambient_strength = 0.15;
    var total_light = albedo * ambient_strength;

    for (var i = 0u; i < 4u; i = i + 1u) {
        let light = u.lights[i];
        let intensity = light.color.w;

        if (intensity <= 0.0) {
            continue;
        }

        let light_color = light.color.rgb;
        let L = light.pos.xyz - in.world_pos;
        let distance = length(L);
        let L_dir = L / distance;

        let attenuation = intensity / (1.0 + 0.09 * distance + 0.032 * distance * distance);

        let diff = max(dot(N, L_dir), 0.0);
        let diffuse = diff * light_color * attenuation;

        let H = normalize(L_dir + V);
        let spec = pow(max(dot(N, H), 0.0), 32.0);
        let specular = spec * light_color * attenuation * 0.5;

        total_light = total_light + albedo * diffuse + specular;
    }

    return vec4<f32>(total_light, 1.0);
}
