// Sprint 257: GPU-accelerated UI panel hit-testing via compute shader
// @group(0) @binding(0): array<vec4<f32>> — panel AABBs as [min_x, min_y, max_x, max_y]
// @group(0) @binding(1): vec2<f32> — mouse position in screen coords (pixels)
// Output is written to the positions buffer as the hit panel index (f32 cast).

@group(0) @binding(0) var<storage, read_write> panels: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> mouse_pos: vec2<f32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let panel_count = arrayLength(&panels);
    var hit_index: i32 = -1;

    for (var i = 0u; i < panel_count; i++) {
        let bb = panels[i];
        let mx = mouse_pos.x;
        let my = mouse_pos.y;
        if (mx >= bb.x && mx <= bb.z && my >= bb.y && my <= bb.w) {
            hit_index = i32(i);
        }
    }

    // Write hit result to first element (overloaded for result readback)
    panels[0] = vec4(f32(hit_index), 0.0, 0.0, 0.0);
}
