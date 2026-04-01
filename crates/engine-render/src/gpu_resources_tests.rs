use engine_assets::MeshVertex;

use crate::gpu_resources::{build_gpu_vertices, compute_index_count};

#[test]
fn build_gpu_vertices_preserves_vertex_attributes() {
    let vertices = vec![
        MeshVertex {
            position: [1.0, 2.0, 3.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.25, 0.75],
        },
        MeshVertex {
            position: [-1.0, -2.0, -3.0],
            normal: [1.0, 0.0, 0.0],
            uv: [1.0, 0.0],
        },
    ];

    let gpu_vertices = build_gpu_vertices(vertices.as_slice());

    assert_eq!(gpu_vertices.len(), vertices.len());
    assert_eq!(gpu_vertices[0].position, vertices[0].position);
    assert_eq!(gpu_vertices[0].normal, vertices[0].normal);
    assert_eq!(gpu_vertices[0].uv, vertices[0].uv);
    assert_eq!(gpu_vertices[1].position, vertices[1].position);
    assert_eq!(gpu_vertices[1].normal, vertices[1].normal);
    assert_eq!(gpu_vertices[1].uv, vertices[1].uv);
}

#[test]
fn compute_index_count_accepts_valid_lengths() {
    let index_count = compute_index_count(42).expect("valid index count should pass");

    assert_eq!(index_count, 42);
}

#[test]
fn compute_index_count_rejects_overflow_lengths() {
    if usize::BITS <= 32 {
        return;
    }

    let overflow_len = (u32::MAX as usize) + 1;
    let error = compute_index_count(overflow_len).expect_err("index count beyond u32 should fail");

    assert!(error.to_string().contains("overflow"));
}
