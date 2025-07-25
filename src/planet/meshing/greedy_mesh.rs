use crate::planet::meshing::{mesh_renderer::VOXEL_SIZE, dual_contourer::CellMesh};
use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct GreedyQuad {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

pub struct GreedyMeshHandler;

impl GreedyMeshHandler {
    /// Generates a greedy mesh from a list of quads defined by their coordinates.
    /// The quads are defined by their top-left corner (x, y) and the function
    /// will create larger rectangles where possible to optimize the mesh.
    pub fn greedy_mesh_quads(
        quads: &[(i32, i32)],
        min_x: i32,
        min_y: i32,
        max_x: i32,
        max_y: i32,
    ) -> Vec<GreedyQuad> {
        let mut result = Vec::new();
        let width = (max_x - min_x) as usize;
        let height = (max_y - min_y) as usize;

        // Create a grid to mark which quads have been processed
        let mut processed = vec![vec![false; width]; height];
        let mut quad_grid = vec![vec![false; width]; height];

        // Mark existing quads in the grid
        for &(qx, qy) in quads {
            let gx = (qx - min_x) as usize;
            let gy = (qy - min_y) as usize;
            if gx < width && gy < height {
                quad_grid[gy][gx] = true;
            }
        }

        // Greedy meshing algorithm
        for y in 0..height {
            for x in 0..width {
                if !processed[y][x] && quad_grid[y][x] {
                    // Start a new greedy quad
                    let mut quad_width = 1;
                    let mut quad_height = 1;

                    // Extend horizontally as much as possible
                    while x + quad_width < width
                        && !processed[y][x + quad_width]
                        && quad_grid[y][x + quad_width]
                    {
                        quad_width += 1;
                    }

                    // Try to extend vertically
                    'vertical_loop: while y + quad_height < height {
                        // Check if the entire horizontal strip is available
                        for dx in 0..quad_width {
                            if processed[y + quad_height][x + dx]
                                || !quad_grid[y + quad_height][x + dx]
                            {
                                break 'vertical_loop;
                            }
                        }
                        quad_height += 1;
                    }

                    // Mark all quads in this rectangle as processed
                    for dy in 0..quad_height {
                        for dx in 0..quad_width {
                            processed[y + dy][x + dx] = true;
                        }
                    }

                    // Add the greedy quad
                    result.push(GreedyQuad {
                        x: min_x + x as i32,
                        y: min_y + y as i32,
                        width: quad_width as i32,
                        height: quad_height as i32,
                    });
                }
            }
        }

        result
    }

    pub fn is_full_quad_mesh(mesh: &CellMesh, x: f32, y: f32) -> bool {
        // A full quad should have exactly 4 vertices and 2 triangles
        if mesh.vertices.len() != 4 || mesh.triangles.len() != 2 {
            return false;
        }

        // Check if vertices form a unit square
        let expected_vertices = [
            Vec2::new(x, y),
            Vec2::new(x + 1.0, y),
            Vec2::new(x + 1.0, y + 1.0),
            Vec2::new(x, y + 1.0),
        ];

        // Vertices might be in different order, so check if all expected vertices exist
        for expected in &expected_vertices {
            if !mesh
                .vertices
                .iter()
                .any(|v| (v.x - expected.x).abs() < 0.001 && (v.y - expected.y).abs() < 0.001)
            {
                return false;
            }
        }

        true
    }

    /// Add a greedy quad to the mesh, aka. a bigger rectangle.
    pub fn add_quad_to_mesh(
        vertices: &mut Vec<[f32; 3]>,
        indices: &mut Vec<u32>,
        quad: GreedyQuad,
    ) {
        let vertex_offset = vertices.len() as u32;

        // Add vertices for the rectangle
        let x = quad.x as f32;
        let y = quad.y as f32;
        let w = quad.width as f32;
        let h = quad.height as f32;

        vertices.extend_from_slice(&[
            [x * VOXEL_SIZE, y * VOXEL_SIZE, 0.0], // bottom-left
            [(x + w) * VOXEL_SIZE, y * VOXEL_SIZE, 0.0], // bottom-right
            [
                (x + w) * VOXEL_SIZE,
                (y + h) * VOXEL_SIZE,
                0.0,
            ], // top-right
            [x * VOXEL_SIZE, (y + h) * VOXEL_SIZE, 0.0], // top-left
        ]);

        // Add indices for two triangles
        indices.extend_from_slice(&[
            vertex_offset + 0,
            vertex_offset + 1,
            vertex_offset + 2, // first triangle
            vertex_offset + 0,
            vertex_offset + 2,
            vertex_offset + 3, // second triangle
        ]);
    }
}
