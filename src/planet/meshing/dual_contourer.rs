use bevy::prelude::*;

use crate::planet::{
    meshing::{greedy_mesh::GreedyMeshHandler, mesh_renderer::VOXEL_SIZE},
    world::{chunk_world::World, voxel::VoxelType},
};

/// Dual contouring algorithm for generating meshes from voxel data
pub struct DualContourer;

#[derive(Debug)]
struct CellConfiguration {
    // Corner SDF values: bottom-left, bottom-right, top-right, top-left
    corners: [f32; 4],
}

#[derive(Debug)]
pub struct CellMesh {
    pub vertices: Vec<Vec2>,
    pub triangles: Vec<[u32; 3]>,
}

impl DualContourer {
    pub fn generate_chunk_mesh(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        target_type: VoxelType,
    ) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let chunk_size = World::chunk_size() as i32;

        // Calculate world bounds for this specific chunk
        let world_min_x = chunk_x * chunk_size;
        let world_max_x = (chunk_x + 1) * chunk_size;
        let world_min_y = chunk_y * chunk_size;
        let world_max_y = (chunk_y + 1) * chunk_size;

        // Generate dual contour mesh data for this chunk
        let mut complex_meshes = Vec::new();
        let mut mesh_quads = Vec::new();

        // Generate cells for this chunk, including boundary cells for seamless connection
        // We need to generate cells for the boundaries to ensure seamless edges
        for world_y in world_min_y..world_max_y {
            for world_x in world_min_x..world_max_x {
                // Check if this cell "belongs" to this chunk to avoid duplicates
                // A cell belongs to the chunk containing its bottom-left corner
                let cell_chunk_x = world_x.div_euclid(chunk_size);
                let cell_chunk_y = world_y.div_euclid(chunk_size);

                // Only process cells that belong to this chunk
                if cell_chunk_x == chunk_x && cell_chunk_y == chunk_y {
                    // Try to create cells even if neighbors aren't loaded
                    // For boundary cells, we'll use Air as default for missing voxels
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

        // Apply greedy meshing to full quads within this chunk (disabled for SDF)
        let greedy_quads = GreedyMeshHandler::greedy_mesh_quads(
            &mesh_quads,
            world_min_x,
            world_min_y,
            world_max_x,
            world_max_y,
        );

        // Add greedy meshed quads to the final mesh (disabled for SDF)
        for quad in greedy_quads {
            GreedyMeshHandler::add_quad_to_mesh(&mut vertices, &mut indices, quad);
        }

        // Add complex meshes to the final mesh
        for cell_mesh in complex_meshes {
            let vertex_offset = vertices.len() as u32;

            // Add vertices with proper world positioning
            for vertex in cell_mesh.vertices {
                vertices.push([vertex.x * VOXEL_SIZE, vertex.y * VOXEL_SIZE, 0.0]);
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
        world: &World,
        x: i32,
        y: i32,
        target_type: VoxelType,
    ) -> CellConfiguration {
        let chunk_size = World::chunk_size() as i32;

        // Helper function to safely get SDF value, with intelligent defaults for chunk boundaries
        let safe_get_sdf = |vx: i32, vy: i32| -> f32 {
            let chunk_x = vx.div_euclid(chunk_size);
            let chunk_y = vy.div_euclid(chunk_size);

            if world.is_chunk_loaded(chunk_x, chunk_y) {
                let sdf = world.get_voxel_sdf(vx, vy);
                // For target type matching, we want negative values where the target material exists
                if sdf.get_material() == target_type {
                    -sdf.distance.abs() // Make negative for target material
                } else {
                    sdf.distance.abs() // Make positive for non-target material
                }
            } else {
                // For missing chunks, try to make a reasonable assumption
                // Check if the current cell's chunk has the target material at the boundary
                // and assume continuity across chunk boundaries

                let current_chunk_x = x.div_euclid(chunk_size);
                let current_chunk_y = y.div_euclid(chunk_size);

                if world.is_chunk_loaded(current_chunk_x, current_chunk_y) {
                    let boundary_x = if vx < current_chunk_x * chunk_size {
                        current_chunk_x * chunk_size
                    } else if vx >= (current_chunk_x + 1) * chunk_size {
                        (current_chunk_x + 1) * chunk_size - 1
                    } else {
                        vx
                    };

                    let boundary_y = if vy < current_chunk_y * chunk_size {
                        current_chunk_y * chunk_size
                    } else if vy >= (current_chunk_y + 1) * chunk_size {
                        (current_chunk_y + 1) * chunk_size - 1
                    } else {
                        vy
                    };

                    let boundary_sdf = world.get_voxel_sdf(boundary_x, boundary_y);
                    if boundary_sdf.get_material() == target_type {
                        -boundary_sdf.distance.abs() // Assume continuity
                    } else {
                        boundary_sdf.distance.abs()
                    }
                } else {
                    1.0
                }
            }
        };

        CellConfiguration {
            corners: [
                safe_get_sdf(x, y),         // bottom-left
                safe_get_sdf(x + 1, y),     // bottom-right
                safe_get_sdf(x + 1, y + 1), // top-right
                safe_get_sdf(x, y + 1),     // top-left
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

        // Count how many corners are inside (negative SDF)
        let inside_corners: Vec<bool> = corners.iter().map(|&c| c <= 0.0).collect();
        let inside_count = inside_corners.iter().filter(|&&inside| inside).count();

        match inside_count {
            0 => {
                // All corners outside - no mesh needed
                None
            }
            4 => {
                // All corners inside - create full quad but check for nearby surface intersections
                // This ensures interior cells connect properly with surface cells
                Some(Self::create_full_quad(cell_x, cell_y))
            }
            _ => {
                // Partial fill - this is where the surface is, use intersection-based meshing
                let intersections = Self::calculate_edge_intersections(&corners, cell_x, cell_y);
                Self::create_surface_mesh_from_corners(
                    inside_corners,
                    intersections,
                    cell_x,
                    cell_y,
                )
            }
        }
    }

    /// Create a surface mesh that properly handles partial fills with precise intersection points
    fn create_surface_mesh_from_corners(
        inside_corners: Vec<bool>,
        intersections: Vec<Vec2>,
        cell_x: f32,
        cell_y: f32,
    ) -> Option<CellMesh> {
        let corner_positions = [
            Vec2::new(cell_x, cell_y),             // bottom-left
            Vec2::new(cell_x + 1.0, cell_y),       // bottom-right
            Vec2::new(cell_x + 1.0, cell_y + 1.0), // top-right
            Vec2::new(cell_x, cell_y + 1.0),       // top-left
        ];

        let mut vertices = Vec::new();
        let mut triangles = Vec::new();

        // Add all intersection points
        for intersection in &intersections {
            vertices.push(*intersection);
        }

        // Add inside corners (corners that are part of the filled material)
        for (i, &is_inside) in inside_corners.iter().enumerate() {
            if is_inside {
                vertices.push(corner_positions[i]);
            }
        }

        if vertices.len() < 3 {
            return None; // Not enough vertices for a triangle
        }

        // Create a polygon from the vertices
        // For proper dual contouring, we should sort vertices in order around the cell
        let center = Vec2::new(cell_x + 0.5, cell_y + 0.5);
        let mut indexed_vertices: Vec<(usize, Vec2)> =
            vertices.iter().enumerate().map(|(i, &v)| (i, v)).collect();

        // Sort vertices by angle around cell center
        indexed_vertices.sort_by(|a, b| {
            let angle_a = (a.1 - center).y.atan2((a.1 - center).x);
            let angle_b = (b.1 - center).y.atan2((b.1 - center).x);
            angle_a.partial_cmp(&angle_b).unwrap()
        });

        // Create triangle fan from the sorted vertices
        if indexed_vertices.len() >= 3 {
            for i in 1..indexed_vertices.len() - 1 {
                triangles.push([
                    indexed_vertices[0].0 as u32,
                    indexed_vertices[i].0 as u32,
                    indexed_vertices[i + 1].0 as u32,
                ]);
            }
        }

        Some(CellMesh {
            vertices,
            triangles,
        })
    }

    /// Check if there's a surface crossing in this cell
    fn has_surface_crossing(corners: &[f32; 4]) -> bool {
        // Look for sign changes between corners
        for i in 0..4 {
            let next = (i + 1) % 4;
            if (corners[i] > 0.0) != (corners[next] > 0.0) {
                return true;
            }
        }
        false
    }

    /// Calculate where the surface intersects each edge
    fn calculate_edge_intersections(corners: &[f32; 4], cell_x: f32, cell_y: f32) -> Vec<Vec2> {
        let mut intersections = Vec::new();

        // Define corner positions
        let corner_positions = [
            Vec2::new(cell_x, cell_y),             // bottom-left
            Vec2::new(cell_x + 1.0, cell_y),       // bottom-right
            Vec2::new(cell_x + 1.0, cell_y + 1.0), // top-right
            Vec2::new(cell_x, cell_y + 1.0),       // top-left
        ];

        // Check each edge for intersections
        for i in 0..4 {
            let next = (i + 1) % 4;
            let v1 = corners[i];
            let v2 = corners[next];

            // Check for sign change (surface crossing)
            if (v1 > 0.0) != (v2 > 0.0) {
                // Linear interpolation to find intersection point
                let t = v1 / (v1 - v2);
                let intersection = corner_positions[i].lerp(corner_positions[next], t);
                intersections.push(intersection);
            }
        }

        intersections
    }

    /// Create a mesh from intersection points
    fn create_mesh_from_intersections(
        intersections: Vec<Vec2>,
        cell_x: f32,
        cell_y: f32,
    ) -> Option<CellMesh> {
        if intersections.len() < 2 {
            return None;
        }

        if intersections.len() == 2 {
            // Simple case: line segment, create a thin quad
            let p1 = intersections[0];
            let p2 = intersections[1];

            // Calculate perpendicular direction for thickness
            let dir = (p2 - p1).normalize();
            let perp = Vec2::new(-dir.y, dir.x) * 0.1; // Small thickness

            let vertices = vec![p1 + perp, p2 + perp, p2 - perp, p1 - perp];

            return Some(CellMesh {
                vertices,
                triangles: vec![[0, 1, 2], [0, 2, 3]],
            });
        }

        if intersections.len() == 4 {
            // Four intersections - create a quad connecting them
            // Sort intersections clockwise around cell center
            let center = Vec2::new(cell_x + 0.5, cell_y + 0.5);
            let mut sorted_intersections = intersections;
            sorted_intersections.sort_by(|a, b| {
                let angle_a = (a - center).y.atan2((a - center).x);
                let angle_b = (b - center).y.atan2((b - center).x);
                angle_a.partial_cmp(&angle_b).unwrap()
            });

            return Some(CellMesh {
                vertices: sorted_intersections,
                triangles: vec![[0, 1, 2], [0, 2, 3]],
            });
        }

        // For other cases, create a fan from center
        let center = Vec2::new(cell_x + 0.5, cell_y + 0.5);
        let mut vertices = vec![center];
        vertices.extend(intersections);

        let mut triangles = Vec::new();
        for i in 1..vertices.len() {
            let next = if i == vertices.len() - 1 { 1 } else { i + 1 };
            triangles.push([0, i as u32, next as u32]);
        }

        Some(CellMesh {
            vertices,
            triangles,
        })
    }

    // Helper function to check if we can sample all 4 corners of a cell
    // fn can_sample_cell(world: &ChunkWorld, x: i32, y: i32) -> bool {
    //     let chunk_size = ChunkWorld::chunk_size() as i32;

    //     for dy in 0..=1 {
    //         for dx in 0..=1 {
    //             let voxel_x = x + dx;
    //             let voxel_y = y + dy;
    //             let chunk_x = voxel_x.div_euclid(chunk_size);
    //             let chunk_y = voxel_y.div_euclid(chunk_size);

    //             if !world.is_chunk_loaded(chunk_x, chunk_y) {
    //                 return false;
    //             }
    //         }
    //     }
    //     true
    // }
}
