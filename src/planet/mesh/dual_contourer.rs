use bevy::prelude::*;

use crate::planet::mesh::{chunk_world::ChunkWorld, greedy_mesh::GreedyMeshHandler, VoxelType, VOXEL_SIZE};

/// Dual contouring algorithm for generating meshes from voxel data
pub struct DualContourer;

#[derive(Debug)]
struct CellConfiguration {
    // Corners: bottom-left, bottom-right, top-right, top-left
    corners: [bool; 4],
}

#[derive(Debug)]
pub struct CellMesh {
    pub vertices: Vec<Vec2>,
    pub triangles: Vec<[u32; 3]>,
}

impl DualContourer {
    pub fn generate_chunk_mesh(
        world: &ChunkWorld,
        chunk_x: i32,
        chunk_y: i32,
        target_type: VoxelType,
    ) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let chunk_size = ChunkWorld::chunk_size() as i32;

        // Calculate world bounds for this specific chunk
        let world_min_x = chunk_x * chunk_size;
        let world_max_x = (chunk_x + 1) * chunk_size;
        let world_min_y = chunk_y * chunk_size;
        let world_max_y = (chunk_y + 1) * chunk_size;

        // Generate dual contour mesh data for this chunk
        let mut mesh_quads = Vec::new();
        let mut complex_meshes = Vec::new();

        // Generate cells for this chunk, including boundary cells for seamless connection
        // We'll include boundaries but use chunk ownership rules to avoid duplicates
        for world_y in world_min_y..world_max_y {
            for world_x in world_min_x..world_max_x {
                // Check if this cell "belongs" to this chunk to avoid duplicates
                // A cell belongs to the chunk containing its bottom-left corner
                let cell_chunk_x = world_x.div_euclid(chunk_size);
                let cell_chunk_y = world_y.div_euclid(chunk_size);

                // Only process cells that belong to this chunk
                if cell_chunk_x == chunk_x && cell_chunk_y == chunk_y {
                    // Only create cells where we can sample all 4 corners
                    if Self::can_sample_cell(world, world_x, world_y) {
                        let cell = Self::get_cell_configuration(world, world_x, world_y, target_type);

                        if let Some(cell_mesh) =
                            Self::generate_cell_mesh(cell, world_x as f32, world_y as f32)
                        {
                            // Check if this is a simple full quad that can be greedy meshed
                            if GreedyMeshHandler::is_full_quad_mesh(
                                &cell_mesh,
                                world_x as f32,
                                world_y as f32,
                            ) {
                                mesh_quads.push((world_x, world_y));
                            } else {
                                // Complex shapes go directly into the final mesh
                                complex_meshes.push(cell_mesh);
                            }
                        }
                    }
                }
            }
        }

        // Apply greedy meshing to full quads within this chunk
        let greedy_quads = GreedyMeshHandler::greedy_mesh_quads(
            &mesh_quads,
            world_min_x,
            world_min_y,
            world_max_x,
            world_max_y,
        );

        // Add greedy meshed quads to the final mesh
        for quad in greedy_quads {
            GreedyMeshHandler::add_quad_to_mesh(&mut vertices, &mut indices, quad);
        }

        // Add complex meshes to the final mesh
        for cell_mesh in complex_meshes {
            let vertex_offset = vertices.len() as u32;

            // Add vertices
            for vertex in cell_mesh.vertices {
                vertices.push([
                    vertex.x * VOXEL_SIZE - 400.0,
                    300.0 - vertex.y * VOXEL_SIZE,
                    0.0,
                ]);
            }

            // Add indices with offset
            for triangle in cell_mesh.triangles {
                indices.push(vertex_offset + triangle[0]);
                indices.push(vertex_offset + triangle[1]);
                indices.push(vertex_offset + triangle[2]);
            }
        }

        (vertices, indices)
    }

    // v--------- Private methods ----------v //

    fn get_cell_configuration(
        world: &ChunkWorld,
        x: i32,
        y: i32,
        target_type: VoxelType,
    ) -> CellConfiguration {
        CellConfiguration {
            corners: [
                world.get_voxel(x, y) == target_type,         // bottom-left
                world.get_voxel(x + 1, y) == target_type,     // bottom-right
                world.get_voxel(x + 1, y + 1) == target_type, // top-right
                world.get_voxel(x, y + 1) == target_type,     // top-left
            ],
        }
    }

    fn create_full_quad(cell_x: f32, cell_y: f32) -> CellMesh {
        let base_x = cell_x;
        let base_y = cell_y;

        CellMesh {
            vertices: vec![
                Vec2::new(base_x, base_y),
                Vec2::new(base_x + 1.0, base_y),
                Vec2::new(base_x + 1.0, base_y + 1.0),
                Vec2::new(base_x, base_y + 1.0),
            ],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        }
    }

    fn create_corner_mesh(cell_x: f32, cell_y: f32, corner: usize) -> CellMesh {
        let base_x = cell_x;
        let base_y = cell_y;

        match corner {
            0 => CellMesh {
                // bottom-left
                vertices: vec![
                    Vec2::new(base_x, base_y),
                    Vec2::new(base_x + 0.5, base_y),
                    Vec2::new(base_x, base_y + 0.5),
                ],
                triangles: vec![[0, 1, 2]],
            },
            1 => CellMesh {
                // bottom-right
                vertices: vec![
                    Vec2::new(base_x + 1.0, base_y),
                    Vec2::new(base_x + 0.5, base_y),
                    Vec2::new(base_x + 1.0, base_y + 0.5),
                ],
                triangles: vec![[0, 1, 2]],
            },
            2 => CellMesh {
                // top-right
                vertices: vec![
                    Vec2::new(base_x + 1.0, base_y + 1.0),
                    Vec2::new(base_x + 1.0, base_y + 0.5),
                    Vec2::new(base_x + 0.5, base_y + 1.0),
                ],
                triangles: vec![[0, 1, 2]],
            },
            3 => CellMesh {
                // top-left
                vertices: vec![
                    Vec2::new(base_x, base_y + 1.0),
                    Vec2::new(base_x, base_y + 0.5),
                    Vec2::new(base_x + 0.5, base_y + 1.0),
                ],
                triangles: vec![[0, 1, 2]],
            },
            _ => unreachable!(),
        }
    }

    fn create_inverse_corner_mesh(cell_x: f32, cell_y: f32, corner: usize) -> CellMesh {
        let base_x = cell_x;
        let base_y = cell_y;

        // Create full square minus the corner
        match corner {
            0 => CellMesh {
                // all except bottom-left
                vertices: vec![
                    Vec2::new(base_x + 0.5, base_y),
                    Vec2::new(base_x + 1.0, base_y),
                    Vec2::new(base_x + 1.0, base_y + 1.0),
                    Vec2::new(base_x, base_y + 1.0),
                    Vec2::new(base_x, base_y + 0.5),
                ],
                triangles: vec![[0, 1, 2], [0, 2, 3], [0, 3, 4]],
            },
            1 => CellMesh {
                // all except bottom-right (case 13)
                vertices: vec![
                    Vec2::new(base_x, base_y),
                    Vec2::new(base_x + 0.5, base_y),
                    Vec2::new(base_x + 1.0, base_y + 0.5),
                    Vec2::new(base_x + 1.0, base_y + 1.0),
                    Vec2::new(base_x, base_y + 1.0),
                ],
                triangles: vec![[0, 1, 2], [0, 2, 3], [0, 3, 4]],
            },
            2 => CellMesh {
                // all except top-right (case 11)
                vertices: vec![
                    Vec2::new(base_x, base_y),
                    Vec2::new(base_x + 1.0, base_y),
                    Vec2::new(base_x + 1.0, base_y + 0.5),
                    Vec2::new(base_x + 0.5, base_y + 1.0),
                    Vec2::new(base_x, base_y + 1.0),
                ],
                triangles: vec![[0, 1, 2], [0, 2, 3], [0, 3, 4]],
            },
            3 => CellMesh {
                // all except top-left (case 7)
                vertices: vec![
                    Vec2::new(base_x, base_y),
                    Vec2::new(base_x + 1.0, base_y),
                    Vec2::new(base_x + 1.0, base_y + 1.0),
                    Vec2::new(base_x + 0.5, base_y + 1.0),
                    Vec2::new(base_x, base_y + 0.5),
                ],
                triangles: vec![[0, 1, 2], [0, 2, 3], [0, 3, 4]],
            },
            _ => unreachable!(),
        }
    }

    fn create_edge_mesh(cell_x: f32, cell_y: f32, edge: usize) -> CellMesh {
        let base_x = cell_x;
        let base_y = cell_y;

        match edge {
            0 => CellMesh {
                // bottom edge
                vertices: vec![
                    Vec2::new(base_x, base_y),
                    Vec2::new(base_x + 1.0, base_y),
                    Vec2::new(base_x + 1.0, base_y + 0.5),
                    Vec2::new(base_x, base_y + 0.5),
                ],
                triangles: vec![[0, 1, 2], [0, 2, 3]],
            },
            1 => CellMesh {
                // right edge
                vertices: vec![
                    Vec2::new(base_x + 0.5, base_y),
                    Vec2::new(base_x + 1.0, base_y),
                    Vec2::new(base_x + 1.0, base_y + 1.0),
                    Vec2::new(base_x + 0.5, base_y + 1.0),
                ],
                triangles: vec![[0, 1, 2], [0, 2, 3]],
            },
            2 => CellMesh {
                // top edge
                vertices: vec![
                    Vec2::new(base_x, base_y + 0.5),
                    Vec2::new(base_x + 1.0, base_y + 0.5),
                    Vec2::new(base_x + 1.0, base_y + 1.0),
                    Vec2::new(base_x, base_y + 1.0),
                ],
                triangles: vec![[0, 1, 2], [0, 2, 3]],
            },
            3 => CellMesh {
                // left edge
                vertices: vec![
                    Vec2::new(base_x, base_y),
                    Vec2::new(base_x + 0.5, base_y),
                    Vec2::new(base_x + 0.5, base_y + 1.0),
                    Vec2::new(base_x, base_y + 1.0),
                ],
                triangles: vec![[0, 1, 2], [0, 2, 3]],
            },
            _ => unreachable!(),
        }
    }

    fn create_diagonal_mesh(cell_x: f32, cell_y: f32, flip: bool) -> CellMesh {
        let base_x = cell_x;
        let base_y = cell_y;

        if flip {
            // bottom-right + top-left
            CellMesh {
                vertices: vec![
                    Vec2::new(base_x + 0.5, base_y),       // bottom center
                    Vec2::new(base_x + 1.0, base_y),       // bottom-right
                    Vec2::new(base_x + 1.0, base_y + 0.5), // right center
                    Vec2::new(base_x, base_y + 0.5),       // left center
                    Vec2::new(base_x, base_y + 1.0),       // top-left
                    Vec2::new(base_x + 0.5, base_y + 1.0), // top center
                ],
                triangles: vec![
                    [0, 1, 2], // bottom-right triangle
                    [3, 4, 5], // top-left triangle
                ],
            }
        } else {
            // bottom-left + top-right
            CellMesh {
                vertices: vec![
                    Vec2::new(base_x, base_y),             // bottom-left
                    Vec2::new(base_x + 0.5, base_y),       // bottom center
                    Vec2::new(base_x, base_y + 0.5),       // left center
                    Vec2::new(base_x + 0.5, base_y + 1.0), // top center
                    Vec2::new(base_x + 1.0, base_y + 1.0), // top-right
                    Vec2::new(base_x + 1.0, base_y + 0.5), // right center
                ],
                triangles: vec![
                    [0, 1, 2], // bottom-left triangle
                    [3, 4, 5], // top-right triangle
                ],
            }
        }
    }

    fn create_generic_mesh(cell_x: f32, cell_y: f32, corners: [bool; 4]) -> CellMesh {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();

        // Count filled corners
        let filled_count = corners.iter().filter(|&&c| c).count();

        if filled_count == 0 {
            // No mesh needed
            return CellMesh {
                vertices,
                triangles,
            };
        }

        if filled_count == 4 {
            // All filled - create full quad
            let base_x = cell_x;
            let base_y = cell_y;
            vertices = vec![
                Vec2::new(base_x, base_y),
                Vec2::new(base_x + 1.0, base_y),
                Vec2::new(base_x + 1.0, base_y + 1.0),
                Vec2::new(base_x, base_y + 1.0),
            ];
            triangles = vec![[0, 1, 2], [0, 2, 3]];
        } else {
            // For partial fills, create a more conservative mesh
            // This is a fallback that tries to avoid sharp edges
            let base_x = cell_x;
            let base_y = cell_y;
            let center_x = base_x + 0.5;
            let center_y = base_y + 0.5;

            // Create triangular segments for each filled corner
            if corners[0] {
                // bottom-left
                vertices.extend_from_slice(&[
                    Vec2::new(base_x, base_y),
                    Vec2::new(center_x, base_y),
                    Vec2::new(base_x, center_y),
                ]);
                let base_idx = vertices.len() as u32 - 3;
                triangles.push([base_idx, base_idx + 1, base_idx + 2]);
            }
            if corners[1] {
                // bottom-right
                vertices.extend_from_slice(&[
                    Vec2::new(base_x + 1.0, base_y),
                    Vec2::new(center_x, base_y),
                    Vec2::new(base_x + 1.0, center_y),
                ]);
                let base_idx = vertices.len() as u32 - 3;
                triangles.push([base_idx, base_idx + 2, base_idx + 1]);
            }
            if corners[2] {
                // top-right
                vertices.extend_from_slice(&[
                    Vec2::new(base_x + 1.0, base_y + 1.0),
                    Vec2::new(base_x + 1.0, center_y),
                    Vec2::new(center_x, base_y + 1.0),
                ]);
                let base_idx = vertices.len() as u32 - 3;
                triangles.push([base_idx, base_idx + 1, base_idx + 2]);
            }
            if corners[3] {
                // top-left
                vertices.extend_from_slice(&[
                    Vec2::new(base_x, base_y + 1.0),
                    Vec2::new(base_x, center_y),
                    Vec2::new(center_x, base_y + 1.0),
                ]);
                let base_idx = vertices.len() as u32 - 3;
                triangles.push([base_idx, base_idx + 2, base_idx + 1]);
            }
        }

        CellMesh {
            vertices,
            triangles,
        }
    }

    fn generate_cell_mesh(config: CellConfiguration, cell_x: f32, cell_y: f32) -> Option<CellMesh> {
        let corners = config.corners;
        let case_index = (corners[0] as u8)
            | ((corners[1] as u8) << 1)
            | ((corners[2] as u8) << 2)
            | ((corners[3] as u8) << 3);

        match case_index {
            0 => None,                                          // All empty
            15 => Some(Self::create_full_quad(cell_x, cell_y)), // All filled

            // Single corner cases
            1 => Some(Self::create_corner_mesh(cell_x, cell_y, 0)), // bottom-left
            2 => Some(Self::create_corner_mesh(cell_x, cell_y, 1)), // bottom-right
            4 => Some(Self::create_corner_mesh(cell_x, cell_y, 2)), // top-right
            8 => Some(Self::create_corner_mesh(cell_x, cell_y, 3)), // top-left

            // Adjacent corner cases (edges)
            3 => Some(Self::create_edge_mesh(cell_x, cell_y, 0)), // bottom edge (corners 0,1)
            6 => Some(Self::create_edge_mesh(cell_x, cell_y, 1)), // right edge (corners 1,2)
            12 => Some(Self::create_edge_mesh(cell_x, cell_y, 2)), // top edge (corners 2,3)
            9 => Some(Self::create_edge_mesh(cell_x, cell_y, 3)), // left edge (corners 3,0)

            // Diagonal cases
            5 => Some(Self::create_diagonal_mesh(cell_x, cell_y, false)), // bottom-left + top-right
            10 => Some(Self::create_diagonal_mesh(cell_x, cell_y, true)), // bottom-right + top-left

            // Three corner cases (inverse of single corner)
            14 => Some(Self::create_inverse_corner_mesh(cell_x, cell_y, 0)), // all except bottom-left
            13 => Some(Self::create_inverse_corner_mesh(cell_x, cell_y, 1)), // all except bottom-right
            11 => Some(Self::create_inverse_corner_mesh(cell_x, cell_y, 2)), // all except top-right
            7 => Some(Self::create_inverse_corner_mesh(cell_x, cell_y, 3)),  // all except top-left

            _ => {
                // Handle any remaining cases
                Some(Self::create_generic_mesh(cell_x, cell_y, corners))
            }
        }
    }

    /// Helper function to check if we can sample all 4 corners of a cell
    fn can_sample_cell(world: &ChunkWorld, x: i32, y: i32) -> bool {
        let chunk_size = ChunkWorld::chunk_size() as i32;

        for dy in 0..=1 {
            for dx in 0..=1 {
                let voxel_x = x + dx;
                let voxel_y = y + dy;
                let chunk_x = voxel_x.div_euclid(chunk_size);
                let chunk_y = voxel_y.div_euclid(chunk_size);

                if !world.is_chunk_loaded(chunk_x, chunk_y) {
                    return false;
                }
            }
        }
        true
    }
}
