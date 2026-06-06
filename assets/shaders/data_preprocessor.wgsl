// Sprint 238/239: Multi-storage-buffer data preprocessor
// @group(0) @binding(0): particle positions (vec3<f32>)
// @group(0) @binding(1): particle velocities (vec3<f32>)
//
// Each workgroup processes 64 elements in parallel.
// Applies Euler integration: position += velocity * delta_time (16ms)

@group(0) @binding(0) var<storage, read_write> positions: array<vec3<f32>>;
@group(0) @binding(1) var<storage, read_write> velocities: array<vec3<f32>>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= arrayLength(&positions)) {
        return;
    }
    let vel = velocities[idx];
    positions[idx] = positions[idx] + vel * 0.016;
}
