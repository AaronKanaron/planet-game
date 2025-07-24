use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

use crate::{VoxelType, VoxelWorld, planet::greedy_mesh::GreedyMeshHandler};

// Voxel size in world units
pub(crate) const VOXEL_SIZE: f32 = 6.0;
const WIREFRAME_MODE: bool = false; // Enable wireframe mode for debugging
const DEBUG_MODE: bool = false; // Enable debug mode for additional logging

#[derive(Component)]
pub struct Voxel;

#[derive(Component)]
pub struct ChunkMesh {
    pub chunk_x: i32,
    pub chunk_y: i32,
}

pub struct MeshRenderer;

struct DualContourer;

impl MeshRenderer {
    pub fn render_mesh(
        mut commands: Commands,
        mut world: ResMut<VoxelWorld>,
        existing_chunks: Query<(Entity, &ChunkMesh)>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        // Get all dirty chunks
        let dirty_chunks = world.get_dirty_chunks();

        if dirty_chunks.is_empty() {
            return; // Nothing to update
        }

        // Statistics tracking
        let start_time = std::time::Instant::now();
        if DEBUG_MODE {
            println!("=== MESH GENERATION STATISTICS ===");
            println!("Dirty chunks to process: {}", dirty_chunks.len());
        }
        let mut despawned_entities = 0;

        // Despawn entities only for dirty chunks
        for (entity, chunk_mesh) in existing_chunks.iter() {
            if dirty_chunks.contains(&(chunk_mesh.chunk_x, chunk_mesh.chunk_y)) {
                commands.entity(entity).despawn();
                despawned_entities += 1;
            }
        }

        if DEBUG_MODE {
            println!("Despawned {} existing entities", despawned_entities);
        }

        // Statistics for mesh generation
        let mut total_vertices = 0;
        let mut total_indices = 0;
        let mut spawned_entities = 0;
        let mut processed_material_types = 0;

        // Regenerate meshes for dirty chunks only
        for &(chunk_x, chunk_y) in &dirty_chunks {
            if DEBUG_MODE {
                println!("Processing chunk ({}, {})", chunk_x, chunk_y);
            }

            for &material_type in &[VoxelType::Rock, VoxelType::Dirt] {
                let chunk_start_time = std::time::Instant::now();
                let (vertices, indices) =
                    DualContourer::generate_chunk_mesh(&world, chunk_x, chunk_y, material_type);
                let generation_time = chunk_start_time.elapsed();

                if !vertices.is_empty() && !indices.is_empty() {
                    if DEBUG_MODE {
                        println!(
                            "  {}: {} vertices, {} indices ({}ms)",
                            match material_type {
                                VoxelType::Rock => "Rock",
                                VoxelType::Dirt => "Dirt",
                                VoxelType::Air => "Air",
                            },
                            vertices.len(),
                            indices.len(),
                            generation_time.as_millis()
                        );
                    }

                    total_vertices += vertices.len();
                    total_indices += indices.len();
                    processed_material_types += 1;
                    // Create filled mesh
                    let mut filled_mesh = Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::RENDER_WORLD,
                    );

                    filled_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone());

                    // Generate normals (all facing forward for 2D)
                    let normals: Vec<[f32; 3]> =
                        (0..vertices.len()).map(|_| [0.0, 0.0, 1.0]).collect();
                    filled_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());

                    filled_mesh.insert_indices(Indices::U32(indices.clone()));

                    let fill_color = match material_type {
                        VoxelType::Air => Color::srgba(0.0, 0.0, 0.0, 0.0), // Transparent
                        VoxelType::Rock => Color::srgb(0.4, 0.4, 0.4),      // Gray
                        VoxelType::Dirt => Color::srgb(0.6, 0.4, 0.2),      // Brown
                    };

                    // Spawn filled mesh
                    commands.spawn((
                        Mesh2d(meshes.add(filled_mesh)),
                        MeshMaterial2d(materials.add(ColorMaterial::from(fill_color))),
                        Transform::default(),
                        Voxel,
                        ChunkMesh { chunk_x, chunk_y },
                    ));
                    spawned_entities += 1;

                    // Create wireframe mesh
                    if !WIREFRAME_MODE {
                        continue; // Skip wireframe if not enabled
                    }

                    let wireframe_indices = Self::generate_wireframe_indices(&indices);

                    if !wireframe_indices.is_empty() {
                        let mut wireframe_mesh =
                            Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::RENDER_WORLD);

                        wireframe_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
                        wireframe_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                        wireframe_mesh.insert_indices(Indices::U32(wireframe_indices));

                        // Spawn wireframe mesh
                        commands.spawn((
                            Mesh2d(meshes.add(wireframe_mesh)),
                            MeshMaterial2d(
                                materials.add(ColorMaterial::from(Color::srgb(1.0, 1.0, 1.0))),
                            ),
                            Transform::from_xyz(0.0, 0.0, 0.1), // Slightly in front
                            Voxel,
                            ChunkMesh { chunk_x, chunk_y },
                        ));
                        spawned_entities += 1;
                    }
                }
            }

            // Mark chunk as clean after processing
            world.mark_chunk_clean(chunk_x, chunk_y);
        }

        // Print final statistics
        let total_time = start_time.elapsed();
        if DEBUG_MODE {
            println!("=== SUMMARY ===");
            println!("Total time: {}ms", total_time.as_millis());
            println!(
                "Processed {} material types across {} chunks",
                processed_material_types,
                dirty_chunks.len()
            );
            println!(
                "Total vertices: {}, Total indices: {}",
                total_vertices, total_indices
            );
            println!(
                "Spawned {} new entities, Despawned {} old entities",
                spawned_entities, despawned_entities
            );
            println!(
                "Average vertices per chunk: {:.1}",
                if dirty_chunks.len() > 0 {
                    total_vertices as f32 / dirty_chunks.len() as f32
                } else {
                    0.0
                }
            );
            println!("=====================================");
        }
    }

    fn generate_wireframe_indices(triangle_indices: &[u32]) -> Vec<u32> {
        let mut wireframe_indices = Vec::new();

        // Convert each triangle to 3 lines
        for triangle in triangle_indices.chunks(3) {
            if triangle.len() == 3 {
                let a = triangle[0];
                let b = triangle[1];
                let c = triangle[2];

                // Add three lines: a-b, b-c, c-a
                wireframe_indices.extend_from_slice(&[a, b, b, c, c, a]);
            }
        }

        wireframe_indices
    }
}

impl DualContourer {
    fn generate_chunk_mesh(
        world: &VoxelWorld,
        chunk_x: i32,
        chunk_y: i32,
        target_type: VoxelType,
    ) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let chunk_size = VoxelWorld::chunk_size() as i32;

        // Check if chunk is loaded
        if !world.is_chunk_loaded(chunk_x, chunk_y) {
            return (vertices, indices);
        }

        // Calculate world bounds for this specific chunk
        let world_min_x = chunk_x * chunk_size;
        let world_max_x = (chunk_x + 1) * chunk_size;
        let world_min_y = chunk_y * chunk_size;
        let world_max_y = (chunk_y + 1) * chunk_size;

        // Statistics tracking
        let mut cells_processed = 0;
        let mut cells_skipped = 0;
        let mut quads_for_greedy = 0;
        let mut complex_meshes_count = 0;
        let mut voxel_accesses = 0;

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
                        voxel_accesses += 4; // We access 4 corners for each cell
                        let cell =
                            Self::get_cell_configuration(world, world_x, world_y, target_type);
                        cells_processed += 1;

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
                                quads_for_greedy += 1;
                            } else {
                                // Complex shapes go directly into the final mesh
                                complex_meshes.push(cell_mesh);
                                complex_meshes_count += 1;
                            }
                        }
                    } else {
                        cells_skipped += 1;
                    }
                }
            }
        }

        // Apply greedy meshing to full quads within this chunk
        let greedy_quads = GreedyMeshHandler::greedy_mesh_quads(
            &mesh_quads,
            world_min_x,
            world_min_y,
            world_max_x, // Include the full range for seamless boundaries
            world_max_y,
        );

        // Print detailed statistics for this chunk
        let material_name = match target_type {
            VoxelType::Rock => "Rock",
            VoxelType::Dirt => "Dirt",
            VoxelType::Air => "Air",
        };
        if DEBUG_MODE {
            println!(
                "    [{}] Chunk ({},{}) stats:",
                material_name, chunk_x, chunk_y
            );
            println!(
                "      Cells: {} processed, {} skipped",
                cells_processed, cells_skipped
            );
            println!("      Voxel accesses: {}", voxel_accesses);
            println!(
                "      Meshes: {} quads for greedy → {} greedy quads, {} complex",
                quads_for_greedy,
                greedy_quads.len(),
                complex_meshes_count
            );
        }

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

    fn get_cell_configuration(
        world: &VoxelWorld,
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
    fn can_sample_cell(world: &VoxelWorld, x: i32, y: i32) -> bool {
        let chunk_size = VoxelWorld::chunk_size() as i32;

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
