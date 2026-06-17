// ── Sprint 303: gpgpu.rs — GPGPU particle helpers extracted from machine.rs ──
//
// Contains: apply_matrix_to_inputs, split_inputs_to_bindings
// These are pure data-transformation functions operating on RelType slices.
// No VM state, no locks — safe to call from any thread.

use crate::executor::RelType;

/// Applies a SIMD `glam::Mat4` transformation in-place to a flat particle buffer.
///
/// Detects stride automatically (6 = pos+vel, 7 = pos+vel+mass).
/// Particle positions (indices 0–2) are transformed via `transform_point3`.
/// Particle velocities (indices 3–5) are transformed via `transform_vector3`.
/// Non-finite results are clamped to 0.0. No allocation.
pub fn apply_matrix_to_inputs(inputs: &mut [RelType], matrix: &glam::Mat4) {
    let len = inputs.len();
    let stride = if len >= 6 && len.is_multiple_of(6) {
        6
    } else if len >= 7 && len.is_multiple_of(7) {
        7
    } else {
        return;
    };
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
        let pos = matrix.transform_point3(glam::Vec3::new(x, y, z));
        inputs[i] = RelType::Float(if pos.x.is_finite() { pos.x as f64 } else { 0.0 });
        inputs[i + 1] = RelType::Float(if pos.y.is_finite() { pos.y as f64 } else { 0.0 });
        inputs[i + 2] = RelType::Float(if pos.z.is_finite() { pos.z as f64 } else { 0.0 });

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
            let vel = matrix.transform_vector3(glam::Vec3::new(vx, vy, vz));
            inputs[i + 3] = RelType::Float(if vel.x.is_finite() { vel.x as f64 } else { 0.0 });
            inputs[i + 4] = RelType::Float(if vel.y.is_finite() { vel.y as f64 } else { 0.0 });
            inputs[i + 5] = RelType::Float(if vel.z.is_finite() { vel.z as f64 } else { 0.0 });
        }
        i += stride;
    }
}

/// Splits a flat interleaved particle buffer into `[positions, velocities]` bindings.
///
/// Stride is auto-detected (6 or 7). Returns `None` if the buffer doesn't align.
/// Allocates two output vectors — use only outside the hot-path.
pub fn split_inputs_to_bindings(inputs: &[RelType]) -> Option<Vec<Vec<RelType>>> {
    let len = inputs.len();
    let stride = if len >= 6 && len.is_multiple_of(6) {
        6
    } else if len >= 7 && len.is_multiple_of(7) {
        7
    } else {
        return None;
    };
    let particle_count = len / stride;
    let mut positions: Vec<RelType> = Vec::with_capacity(particle_count * 3);
    let mut velocities: Vec<RelType> = Vec::with_capacity(particle_count * 3);
    let mut i = 0;
    while i + stride <= len {
        positions.push(inputs[i].clone());
        positions.push(inputs[i + 1].clone());
        positions.push(inputs[i + 2].clone());
        if stride >= 6 {
            velocities.push(inputs[i + 3].clone());
            velocities.push(inputs[i + 4].clone());
            velocities.push(inputs[i + 5].clone());
        }
        i += stride;
    }
    Some(vec![positions, velocities])
}
