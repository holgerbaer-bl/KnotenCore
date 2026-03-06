// mesh3d.wgsl — Sprint 68: Blinn-Phong shading for Mesh3D / Material3D nodes
//
// Bind Group 0:
//   binding 0 → uniform UBO (16 floats view-proj matrix + 8 floats material data)

struct MeshUniforms {
    view_proj: mat4x4<f32>,   // 64 bytes
    material:  vec4<f32>,     // RGBA            (bytes 64-79)
    pbr:       vec4<f32>,     // metallic, roughness, pad, pad (bytes 80-95)
}

@group(0) @binding(0)
var<uniform> u: MeshUniforms;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
}

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)       world_pos: vec3<f32>,
    @location(1)       normal:    vec3<f32>,
}

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    // Model matrix is identity for now – objects are placed at origin.
    // Agents can add a model transform node in a future sprint.
    let world_pos4 = vec4<f32>(v.position, 1.0);
    out.clip_pos  = u.view_proj * world_pos4;
    out.world_pos = v.position;
    out.normal    = normalize(v.normal);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let albedo    = u.material.rgb;
    let alpha     = u.material.a;

    // Simple directional light from camera-upper-right
    let light_dir = normalize(vec3<f32>(1.0, 2.0, 1.5));
    let normal    = normalize(in.normal);

    // Ambient
    let ambient = albedo * 0.15;

    // Diffuse (Blinn-Phong)
    let n_dot_l   = max(dot(normal, light_dir), 0.0);
    let diffuse   = albedo * n_dot_l * 0.75;

    // Specular
    let view_dir  = normalize(-in.world_pos);
    let half_dir  = normalize(light_dir + view_dir);
    let roughness = max(u.pbr.y, 0.01);
    let shininess = 2.0 / (roughness * roughness + 0.001) - 2.0;
    let spec_val  = pow(max(dot(normal, half_dir), 0.0), shininess);
    let metallic  = u.pbr.x;
    let specular  = mix(vec3<f32>(0.04), albedo, metallic) * spec_val * 0.5;

    let out_color = ambient + diffuse + specular;
    return vec4<f32>(clamp(out_color, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);
}
