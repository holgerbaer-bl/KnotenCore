// ── Sprint 303: gpgpu.rs — GPGPU particle helpers extracted from machine.rs ──
//
// Contains: apply_matrix_to_inputs, split_inputs_to_bindings
// These are pure data-transformation functions operating on RelType slices.
// No VM state, no locks — safe to call from any thread.

use crate::executor::RelType;

/// Applies a SIMD 4x4 matrix transformation in-place to a flat particle buffer.
///
/// Detects stride automatically (6 = pos+vel, 7 = pos+vel+mass).
/// Particle positions (indices 0–2) are transformed as 3D points (w=1.0).
/// Particle velocities (indices 3–5) are transformed as 3D vectors (w=0.0).
/// Non-finite results are clamped to 0.0. No allocation.
pub fn apply_matrix_to_inputs(inputs: &mut [RelType], matrix: &[[f32; 4]; 4]) {
    let len = inputs.len();
    let stride = if len >= 6 && len.is_multiple_of(6) {
        6
    } else if len >= 7 && len.is_multiple_of(7) {
        7
    } else {
        return;
    };
    let m = matrix;
    let mut i = 0;
    while i + stride <= len {
        let x = match inputs[i] {
            RelType::Float(f) => f as f32,
            RelType::Int(v) => v as f32,
            _ => {
                i += stride;
                continue;
            }
        };
        let y = match inputs[i + 1] {
            RelType::Float(f) => f as f32,
            RelType::Int(v) => v as f32,
            _ => {
                i += stride;
                continue;
            }
        };
        let z = match inputs[i + 2] {
            RelType::Float(f) => f as f32,
            RelType::Int(v) => v as f32,
            _ => {
                i += stride;
                continue;
            }
        };
        let px = m[0][0] * x + m[1][0] * y + m[2][0] * z + m[3][0];
        let py = m[0][1] * x + m[1][1] * y + m[2][1] * z + m[3][1];
        let pz = m[0][2] * x + m[1][2] * y + m[2][2] * z + m[3][2];

        inputs[i] = RelType::Float(if px.is_finite() { px as f64 } else { 0.0 });
        inputs[i + 1] = RelType::Float(if py.is_finite() { py as f64 } else { 0.0 });
        inputs[i + 2] = RelType::Float(if pz.is_finite() { pz as f64 } else { 0.0 });

        if stride >= 6 {
            let vx = match inputs[i + 3] {
                RelType::Float(f) => f as f32,
                RelType::Int(v) => v as f32,
                _ => {
                    i += stride;
                    continue;
                }
            };
            let vy = match inputs[i + 4] {
                RelType::Float(f) => f as f32,
                RelType::Int(v) => v as f32,
                _ => {
                    i += stride;
                    continue;
                }
            };
            let vz = match inputs[i + 5] {
                RelType::Float(f) => f as f32,
                RelType::Int(v) => v as f32,
                _ => {
                    i += stride;
                    continue;
                }
            };
            let vx_out = m[0][0] * vx + m[1][0] * vy + m[2][0] * vz;
            let vy_out = m[0][1] * vx + m[1][1] * vy + m[2][1] * vz;
            let vz_out = m[0][2] * vx + m[1][2] * vy + m[2][2] * vz;

            inputs[i + 3] = RelType::Float(if vx_out.is_finite() {
                vx_out as f64
            } else {
                0.0
            });
            inputs[i + 4] = RelType::Float(if vy_out.is_finite() {
                vy_out as f64
            } else {
                0.0
            });
            inputs[i + 5] = RelType::Float(if vz_out.is_finite() {
                vz_out as f64
            } else {
                0.0
            });
        }
        i += stride;
    }
}

/// Splits a flat interleaved particle buffer into `[positions, velocities]` bindings.
///
/// If input format is invalid (len < 6 or not a multiple of 6 or 7), returns empty vector.
pub fn split_inputs_to_bindings(inputs: &[RelType]) -> Vec<Vec<RelType>> {
    let len = inputs.len();
    if len < 6 || (!len.is_multiple_of(6) && !len.is_multiple_of(7)) {
        return vec![];
    }
    let stride = if len.is_multiple_of(6) { 6 } else { 7 };
    let particle_count = len / stride;

    let mut pos_binding = Vec::with_capacity(particle_count * 3);
    let mut vel_binding = Vec::with_capacity(particle_count * 3);

    let mut i = 0;
    while i < len {
        pos_binding.push(inputs[i].clone());
        pos_binding.push(inputs[i + 1].clone());
        pos_binding.push(inputs[i + 2].clone());

        vel_binding.push(inputs[i + 3].clone());
        vel_binding.push(inputs[i + 4].clone());
        vel_binding.push(inputs[i + 5].clone());

        i += stride;
    }

    vec![pos_binding, vel_binding]
}
