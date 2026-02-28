struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

@group(1) @binding(0) var t_atlas: texture_2d<f32>;
@group(1) @binding(1) var s_atlas: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct InstanceInput {
    @location(3) instance_pos_and_id: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    
    // instance_pos_and_id contains [X, Y, Z, BlockID]
    let world_position = model.position + instance.instance_pos_and_id.xyz;
    out.clip_position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.normal = model.normal;

    // UV Atlas Calculation
    let block_id = instance.instance_pos_and_id.w;
    
    // We assume an atlas of 16x16 tiles (256 tiles total).
    // block_id = 0 -> UV tile (0, 0)
    // block_id = 1 -> UV tile (1, 0)
    let atlas_size_tiles = 16.0;
    
    let tile_x = block_id % atlas_size_tiles;
    let tile_y = floor(block_id / atlas_size_tiles);
    
    let uv_offset = vec2<f32>(tile_x, tile_y) * (1.0 / atlas_size_tiles);
    let scaled_uv = model.uv * (1.0 / atlas_size_tiles);

    out.tex_coords = scaled_uv + uv_offset;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    
    let tex_color = textureSample(t_atlas, s_atlas, in.tex_coords);

    // Alpha test - drop invisible pixels
    if (tex_color.a < 0.1) {
        discard;
    }

    // Simple directional lighting based on normal
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let diffuse = max(dot(in.normal, light_dir), 0.2);
    let ambient = 0.4;
    
    let final_color = tex_color.rgb * (diffuse + ambient);
    
    return vec4<f32>(final_color, tex_color.a);
}
