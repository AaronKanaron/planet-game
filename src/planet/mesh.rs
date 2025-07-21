use bevy::{asset::RenderAssetUsages, prelude::*, render::mesh::{Indices, PrimitiveTopology}};

use crate::{VoxelType, VoxelWorld};

// Voxel size in world units
pub(crate) const VOXEL_SIZE: f32 = 8.0;

#[derive(Component)]
pub struct Voxel;

pub fn render_mesh(
    mut commands: Commands,
    world: Res<VoxelWorld>,
    existing_voxels: Query<Entity, With<Voxel>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Clean up existing entities
    for entity in existing_voxels.iter() {
        commands.entity(entity).despawn();
    }

    // Create separate meshes for different material types
    for &material_type in &[VoxelType::Rock, VoxelType::Dirt] {
        let (vertices, indices) = generate_dual_contour_mesh(&world, material_type);

        if !vertices.is_empty() && !indices.is_empty() {
            // Create filled mesh
            let mut filled_mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            );

            filled_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone());

            // Generate normals (all facing forward for 2D)
            let normals: Vec<[f32; 3]> = (0..vertices.len()).map(|_| [0.0, 0.0, 1.0]).collect();
            filled_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());

            filled_mesh.insert_indices(Indices::U32(indices.clone()));

            let fill_color = match material_type {
                VoxelType::Air => Color::srgba(0.0, 0.0, 0.0, 0.0), // Transparent
                VoxelType::Rock => Color::srgb(0.4, 0.4, 0.4),      // Gray
                VoxelType::Dirt => Color::srgb(0.6, 0.4, 0.2),      // Brown
                                                                     // VoxelType::Sand => Color::srgb(0.9, 0.8, 0.5),      // Sandy yellow
                                                                     // VoxelType::Water => Color::srgb(0.2, 0.4, 0.8),     // Blue
                                                                     // VoxelType::Ice => Color::srgb(0.8, 0.9, 1.0),       // Light blue-white
                                                                     // VoxelType::Lava => Color::srgb(1.0, 0.3, 0.0),      // Bright orange-red
                                                                     // VoxelType::DeepRock => Color::srgb(0.2, 0.2, 0.3),  // Dark gray-blue
                                                                     // VoxelType::Ore => Color::srgb(0.6, 0.5, 0.2),       // Metallic bronze
                                                                     // VoxelType::Crystal => Color::srgb(0.8, 0.2, 0.9),   // Bright purple
                                                                     // VoxelType::Obsidian => Color::srgb(0.1, 0.1, 0.1),  // Very dark gray/black
                                                                     // VoxelType::Grass => Color::srgb(0.3, 0.7, 0.2),     // Green
            };

            // Spawn filled mesh
            commands.spawn((
                Mesh2d(meshes.add(filled_mesh)),
                MeshMaterial2d(materials.add(ColorMaterial::from(fill_color))),
                Transform::default(), // Scale for visibility
                Voxel,
            ));

            // Create wireframe mesh
            let wireframe_indices = generate_wireframe_indices(&indices);

            if !wireframe_indices.is_empty() {
                let mut wireframe_mesh = Mesh::new(
                    PrimitiveTopology::LineList,
                    RenderAssetUsages::RENDER_WORLD,
                );

                wireframe_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
                wireframe_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                wireframe_mesh.insert_indices(Indices::U32(wireframe_indices));

                // Spawn wireframe mesh
                commands.spawn((
                    Mesh2d(meshes.add(wireframe_mesh)),
                    MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(1.0, 1.0, 1.0)))),
                    Transform::from_xyz(0.0, 0.0, 0.1), // Slightly in front
                    Voxel,
                ));
            }
        }
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

fn generate_dual_contour_mesh(
    world: &VoxelWorld,
    target_type: VoxelType,
) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Process each 2x2 cell in the grid
    //THIS RUNS EVERY FRAME ON EACH VOXEL
    for y in 0..(world.height - 1) {
        for x in 0..(world.width - 1) {
            let cell = get_cell_configuration(world, x as i32, y as i32, target_type);

            if let Some(cell_mesh) = generate_cell_mesh(cell, x as f32, y as f32) {
                let vertex_offset = vertices.len() as u32;

                // Add vertices
                for vertex in cell_mesh.vertices {
                    vertices.push([vertex.x * VOXEL_SIZE - 400.0, 300.0 - vertex.y * VOXEL_SIZE, 0.0]);
                }

                // Add indices with offset
                for triangle in cell_mesh.triangles {
                    indices.push(vertex_offset + triangle[0]);
                    indices.push(vertex_offset + triangle[1]);
                    indices.push(vertex_offset + triangle[2]);
                }
            }
        }
    }

    (vertices, indices)
}

#[derive(Debug)]
struct CellConfiguration {
    // Corners: bottom-left, bottom-right, top-right, top-left
    corners: [bool; 4],
}

#[derive(Debug)]
struct CellMesh {
    vertices: Vec<Vec2>,
    triangles: Vec<[u32; 3]>,
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

fn generate_cell_mesh(config: CellConfiguration, cell_x: f32, cell_y: f32) -> Option<CellMesh> {
    let corners = config.corners;
    let case_index = (corners[0] as u8)
        | ((corners[1] as u8) << 1)
        | ((corners[2] as u8) << 2)
        | ((corners[3] as u8) << 3);

    match case_index {
        0 => None,                                    // All empty
        15 => Some(create_full_quad(cell_x, cell_y)), // All filled

        // Single corner cases
        1 => Some(create_corner_mesh(cell_x, cell_y, 0)), // bottom-left
        2 => Some(create_corner_mesh(cell_x, cell_y, 1)), // bottom-right
        4 => Some(create_corner_mesh(cell_x, cell_y, 2)), // top-right
        8 => Some(create_corner_mesh(cell_x, cell_y, 3)), // top-left

        // Adjacent corner cases (edges)
        3 => Some(create_edge_mesh(cell_x, cell_y, 0)), // bottom edge (corners 0,1)
        6 => Some(create_edge_mesh(cell_x, cell_y, 1)), // right edge (corners 1,2)
        12 => Some(create_edge_mesh(cell_x, cell_y, 2)), // top edge (corners 2,3)
        9 => Some(create_edge_mesh(cell_x, cell_y, 3)), // left edge (corners 3,0)

        // Diagonal cases
        5 => Some(create_diagonal_mesh(cell_x, cell_y, false)), // bottom-left + top-right
        10 => Some(create_diagonal_mesh(cell_x, cell_y, true)), // bottom-right + top-left

        // Three corner cases (inverse of single corner)
        14 => Some(create_inverse_corner_mesh(cell_x, cell_y, 0)), // all except bottom-left
        13 => Some(create_inverse_corner_mesh(cell_x, cell_y, 1)), // all except bottom-right
        11 => Some(create_inverse_corner_mesh(cell_x, cell_y, 2)), // all except top-right
        7 => Some(create_inverse_corner_mesh(cell_x, cell_y, 3)),  // all except top-left

        _ => {
            // Handle any remaining cases
            Some(create_generic_mesh(cell_x, cell_y, corners))
        }
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
        return CellMesh { vertices, triangles };
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
        if corners[0] { // bottom-left
            vertices.extend_from_slice(&[
                Vec2::new(base_x, base_y),
                Vec2::new(center_x, base_y),
                Vec2::new(base_x, center_y),
            ]);
            let base_idx = vertices.len() as u32 - 3;
            triangles.push([base_idx, base_idx + 1, base_idx + 2]);
        }
        if corners[1] { // bottom-right
            vertices.extend_from_slice(&[
                Vec2::new(base_x + 1.0, base_y),
                Vec2::new(center_x, base_y),
                Vec2::new(base_x + 1.0, center_y),
            ]);
            let base_idx = vertices.len() as u32 - 3;
            triangles.push([base_idx, base_idx + 2, base_idx + 1]);
        }
        if corners[2] { // top-right
            vertices.extend_from_slice(&[
                Vec2::new(base_x + 1.0, base_y + 1.0),
                Vec2::new(base_x + 1.0, center_y),
                Vec2::new(center_x, base_y + 1.0),
            ]);
            let base_idx = vertices.len() as u32 - 3;
            triangles.push([base_idx, base_idx + 1, base_idx + 2]);
        }
        if corners[3] { // top-left
            vertices.extend_from_slice(&[
                Vec2::new(base_x, base_y + 1.0),
                Vec2::new(base_x, center_y),
                Vec2::new(center_x, base_y + 1.0),
            ]);
            let base_idx = vertices.len() as u32 - 3;
            triangles.push([base_idx, base_idx + 2, base_idx + 1]);
        }
    }

    CellMesh { vertices, triangles }
}
