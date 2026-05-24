// Sprint 187: Data Preprocessor — GPU-accelerated data normalization shader
// Reads a float array from a storage buffer, applies preprocessing, writes back.
//
// Usage: load this shader, then dispatch_compute(shader_handle, workgroups, 1, 1, data_array)
// Each workgroup processes 64 elements in parallel.

@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= arrayLength(&data)) {
        return;
    }

    // Read the raw value
    let raw = data[idx];

    // Apply preprocessing: clamp to [0, 1] range, then scale
    let clamped = clamp(raw, 0.0, 1.0);
    let normalized = clamped * 100.0;

    // Write back
    data[idx] = normalized;
}
