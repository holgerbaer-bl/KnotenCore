//! Sprint 184: Geometry generators extracted from the registry monolith.
//!
//! Pure mathematical generators for cube, UV-sphere, and cylinder meshes.
//! These functions have no dependencies on handles, windows, or GPU state.

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RegistryVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
}

#[cfg(feature = "ui")]
pub struct CachedMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

#[cfg(not(feature = "ui"))]
pub struct CachedMesh {
    pub index_count: u32,
}

pub fn generate_uv_sphere(rings: u32, sectors: u32) -> (Vec<RegistryVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for r in 0..=rings {
        let phi = std::f32::consts::PI * (r as f32 / rings as f32);
        for s in 0..=sectors {
            let theta = 2.0 * std::f32::consts::PI * (s as f32 / sectors as f32);
            let nx = phi.sin() * theta.cos();
            let ny = phi.cos();
            let nz = phi.sin() * theta.sin();
            let u = s as f32 / sectors as f32;
            let v = r as f32 / rings as f32;
            vertices.push(RegistryVertex {
                position: [nx, ny, nz],
                normal: [nx, ny, nz],
                tex_coords: [u, v],
            });
        }
    }

    for r in 0..rings {
        for s in 0..sectors {
            let first = r * (sectors + 1) + s;
            let second = first + sectors + 1;
            indices.push(first);
            indices.push(second);
            indices.push(first + 1);
            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }
    (vertices, indices)
}

pub fn generate_cylinder(segments: u32) -> (Vec<RegistryVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Top center (normal points up)
    vertices.push(RegistryVertex {
        position: [0.0, 0.5, 0.0],
        normal: [0.0, 1.0, 0.0],
        tex_coords: [0.5, 0.5],
    });
    // Bottom center (normal points down)
    vertices.push(RegistryVertex {
        position: [0.0, -0.5, 0.0],
        normal: [0.0, -1.0, 0.0],
        tex_coords: [0.5, 0.5],
    });

    let base_idx_top: u32 = 0;
    let base_idx_bottom: u32 = 1;

    for i in 0..=segments {
        let theta = 2.0 * std::f32::consts::PI * (i as f32 / segments as f32);
        let x = theta.cos();
        let z = theta.sin();
        let u = i as f32 / segments as f32;
        let nx = x;
        let nz = z;
        // Top cap vertex
        vertices.push(RegistryVertex {
            position: [x, 0.5, z],
            normal: [nx, 0.0, nz],
            tex_coords: [u, 0.0],
        });
        // Bottom cap vertex
        vertices.push(RegistryVertex {
            position: [x, -0.5, z],
            normal: [nx, 0.0, nz],
            tex_coords: [u, 1.0],
        });
    }

    for i in 0..segments {
        let top0 = 2 + i * 2;
        let bot0 = top0 + 1;
        let top1 = top0 + 2;
        let bot1 = top1 + 1;

        // Side faces
        indices.push(top0);
        indices.push(bot0);
        indices.push(top1);
        indices.push(bot0);
        indices.push(bot1);
        indices.push(top1);

        // Top cap
        indices.push(base_idx_top);
        indices.push(top1);
        indices.push(top0);

        // Bottom cap
        indices.push(base_idx_bottom);
        indices.push(bot0);
        indices.push(bot1);
    }
    (vertices, indices)
}

/// Sprint 85: Generate a unit cube with per-face flat normals.
pub fn generate_cube() -> (Vec<RegistryVertex>, Vec<u32>) {
    // 6 faces × 4 vertices = 24 vertices; 6 faces × 2 triangles × 3 = 36 indices
    #[allow(clippy::type_complexity)]
    let faces: [([f32; 3], [[f32; 3]; 4], [[f32; 2]; 4]); 6] = [
        // +Y top
        (
            [0.0, 1.0, 0.0],
            [
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ),
        // -Y bottom
        (
            [0.0, -1.0, 0.0],
            [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
            ],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ),
        // +Z front
        (
            [0.0, 0.0, 1.0],
            [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
        // -Z back
        (
            [0.0, 0.0, -1.0],
            [
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
        // +X right
        (
            [1.0, 0.0, 0.0],
            [
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
        // -X left
        (
            [-1.0, 0.0, 0.0],
            [
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
    ];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    for (face_idx, (normal, positions, uvs)) in faces.iter().enumerate() {
        let base = (face_idx * 4) as u32;
        for (pos, uv) in positions.iter().zip(uvs.iter()) {
            vertices.push(RegistryVertex {
                position: *pos,
                normal: *normal,
                tex_coords: *uv,
            });
        }
        // Two CCW triangles per face
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (vertices, indices)
}
