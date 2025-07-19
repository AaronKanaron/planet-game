use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::input::ButtonInput;
use bevy::window::WindowPlugin;
use bevy::{diagnostic::LogDiagnosticsPlugin, prelude::*};
use noise::{NoiseFn, Perlin};

#[derive(Clone, Copy, PartialEq)]
enum VoxelType {
    Air,
    Rock,
    Dirt,
}

#[derive(Resource)]
struct VoxelWorld {
    width: usize,
    height: usize,
    voxels: Vec<VoxelType>,
}

impl VoxelWorld {
    fn new(width: usize, height: usize) -> Self {
        let voxels = vec![VoxelType::Air; width * height];
        Self {
            width,
            height,
            voxels,
        }
    }

    fn get_voxel(&self, x: usize, y: usize) -> VoxelType {
        if x < self.width && y < self.height {
            self.voxels[y * self.width + x]
        } else {
            VoxelType::Air
        }
    }

    fn set_voxel(&mut self, x: usize, y: usize, voxel_type: VoxelType) {
        if x < self.width && y < self.height {
            self.voxels[y * self.width + x] = voxel_type;
        }
    }

    fn get_voxel_safe(&self, x: i32, y: i32) -> VoxelType {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.voxels[(y as usize) * self.width + (x as usize)]
        } else {
            VoxelType::Air
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Planet Game 2".to_string(),
                resolution: (800.0, 600.0).into(),
                ..default()
            }),
            ..Default::default()
        }))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (render_voxels_dual_contouring, handle_input))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    let mut world = VoxelWorld::new(100, 100);
    generate_terrain(&mut world);

    commands.insert_resource(world);
}

fn generate_terrain(world: &mut VoxelWorld) {
    let noise = Perlin::new(42);
    let center_x = world.width as f32 / 2.0;
    let center_y = world.height as f32 / 2.0;
    let radius = 30.0;

    for x in 0..world.width {
        for y in 0..world.height {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < radius {
                let noise_value = noise.get([x as f64 / 10.0, y as f64 / 10.0]);

                if distance < radius - 5.0 {
                    world.set_voxel(x, y, VoxelType::Rock);
                } else if noise_value > 0.0 {
                    world.set_voxel(x, y, VoxelType::Dirt);
                }
            }
        }
    }
}


#[derive(Component)]
struct Voxel;


fn handle_input(
    mut world: ResMut<VoxelWorld>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    if !mouse_input.pressed(MouseButton::Left) {
        return;
    }

    let window = match windows.single() {
        Ok(w) => w,
        Err(_) => return,
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
        let world_x = ((world_pos.x + 400.0) / 8.0) as isize;
        let world_y = ((300.0 - world_pos.y) / 8.0) as isize;

        if world_x >= 0 && world_y >= 0 {
            world.set_voxel(world_x as usize, world_y as usize, VoxelType::Air);
            world.set_voxel((world_x + 1) as usize, (world_y) as usize, VoxelType::Air);
            world.set_voxel(
                (world_x + 1) as usize,
                (world_y + 1) as usize,
                VoxelType::Air,
            );
            world.set_voxel((world_x) as usize, (world_y + 1) as usize, VoxelType::Air);
            world.set_voxel((world_x - 1) as usize, (world_y) as usize, VoxelType::Air);
            world.set_voxel(
                (world_x - 1) as usize,
                (world_y - 1) as usize,
                VoxelType::Air,
            );
            world.set_voxel((world_x) as usize, (world_y - 1) as usize, VoxelType::Air);
            world.set_voxel(
                (world_x + 1) as usize,
                (world_y - 1) as usize,
                VoxelType::Air,
            );
            world.set_voxel(
                (world_x - 1) as usize,
                (world_y + 1) as usize,
                VoxelType::Air,
            );
        }
    }
}

fn render_voxels_dual_contouring(
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
            let mut mesh = Mesh::new(
                bevy::render::render_resource::PrimitiveTopology::TriangleList,
                bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
            );
            
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone());
            
            // Generate normals (all facing forward for 2D)
            let normals: Vec<[f32; 3]> = (0..vertices.len())
                .map(|_| [0.0, 0.0, 1.0])
                .collect();
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            
            mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
            
            let color = match material_type {
                VoxelType::Rock => Color::srgb(0.4, 0.4, 0.4),
                VoxelType::Dirt => Color::srgb(0.6, 0.4, 0.2),
                VoxelType::Air => continue,
            };
            
            commands.spawn((
                Mesh2d(meshes.add(mesh)),
                MeshMaterial2d(materials.add(ColorMaterial::from(color))),
                Transform::default(),
                Voxel,
            ));
        }
    }
}

fn generate_dual_contour_mesh(world: &VoxelWorld, target_type: VoxelType) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    
    // Process each 2x2 cell in the grid
    for y in 0..(world.height - 1) {
        for x in 0..(world.width - 1) {
            let cell = get_cell_configuration(world, x, y, target_type);
            
            if let Some(cell_mesh) = generate_cell_mesh(cell, x as f32, y as f32) {
                let vertex_offset = vertices.len() as u32;
                
                // Add vertices
                for vertex in cell_mesh.vertices {
                    vertices.push([
                        vertex.x * 8.0 - 400.0,
                        300.0 - vertex.y * 8.0,
                        0.0
                    ]);
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

fn get_cell_configuration(world: &VoxelWorld, x: usize, y: usize, target_type: VoxelType) -> CellConfiguration {
    CellConfiguration {
        corners: [
            world.get_voxel(x, y) == target_type,         // bottom-left
            world.get_voxel(x + 1, y) == target_type,     // bottom-right  
            world.get_voxel(x + 1, y + 1) == target_type, // top-right
            world.get_voxel(x, y + 1) == target_type,     // top-left
        ]
    }
}

fn generate_cell_mesh(config: CellConfiguration, cell_x: f32, cell_y: f32) -> Option<CellMesh> {
    let corners = config.corners;
    let case_index = 
        (corners[0] as u8) |
        ((corners[1] as u8) << 1) |
        ((corners[2] as u8) << 2) |
        ((corners[3] as u8) << 3);
    
    match case_index {
        0 | 15 => None, // All empty or all filled
        
        // Single corner cases
        1 => Some(create_corner_mesh(cell_x, cell_y, 0)), // bottom-left
        2 => Some(create_corner_mesh(cell_x, cell_y, 1)), // bottom-right
        4 => Some(create_corner_mesh(cell_x, cell_y, 2)), // top-right
        8 => Some(create_corner_mesh(cell_x, cell_y, 3)), // top-left
        
        // Three corner cases (inverse of single corner)
        14 => Some(create_inverse_corner_mesh(cell_x, cell_y, 0)), // all except bottom-left
        13 => Some(create_inverse_corner_mesh(cell_x, cell_y, 1)), // all except bottom-right
        11 => Some(create_inverse_corner_mesh(cell_x, cell_y, 2)), // all except top-right
        7 => Some(create_inverse_corner_mesh(cell_x, cell_y, 3)),  // all except top-left
        
        // Edge cases
        3 => Some(create_edge_mesh(cell_x, cell_y, 0)), // bottom edge
        6 => Some(create_edge_mesh(cell_x, cell_y, 1)), // right edge
        12 => Some(create_edge_mesh(cell_x, cell_y, 2)), // top edge
        9 => Some(create_edge_mesh(cell_x, cell_y, 3)), // left edge
        
        // Inverse edge cases
        12 => Some(create_inverse_edge_mesh(cell_x, cell_y, 0)), // all except bottom edge
        9 => Some(create_inverse_edge_mesh(cell_x, cell_y, 1)),  // all except right edge
        3 => Some(create_inverse_edge_mesh(cell_x, cell_y, 2)),  // all except top edge
        6 => Some(create_inverse_edge_mesh(cell_x, cell_y, 3)),  // all except left edge
        
        // Diagonal cases - need to handle ambiguity
        5 => Some(create_diagonal_mesh(cell_x, cell_y, false)), // bottom-left + top-right
        10 => Some(create_diagonal_mesh(cell_x, cell_y, true)),  // bottom-right + top-left
        
        _ => {
            // Handle remaining cases with a simple approach
            Some(create_generic_mesh(cell_x, cell_y, corners))
        }
    }
}

fn create_corner_mesh(cell_x: f32, cell_y: f32, corner: usize) -> CellMesh {
    let base_x = cell_x;
    let base_y = cell_y;
    
    match corner {
        0 => CellMesh { // bottom-left
            vertices: vec![
                Vec2::new(base_x, base_y),
                Vec2::new(base_x + 0.5, base_y),
                Vec2::new(base_x, base_y + 0.5),
            ],
            triangles: vec![[0, 1, 2]],
        },
        1 => CellMesh { // bottom-right
            vertices: vec![
                Vec2::new(base_x + 1.0, base_y),
                Vec2::new(base_x + 0.5, base_y),
                Vec2::new(base_x + 1.0, base_y + 0.5),
            ],
            triangles: vec![[0, 1, 2]],
        },
        2 => CellMesh { // top-right
            vertices: vec![
                Vec2::new(base_x + 1.0, base_y + 1.0),
                Vec2::new(base_x + 1.0, base_y + 0.5),
                Vec2::new(base_x + 0.5, base_y + 1.0),
            ],
            triangles: vec![[0, 1, 2]],
        },
        3 => CellMesh { // top-left
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
        0 => CellMesh { // all except bottom-left
            vertices: vec![
                Vec2::new(base_x + 0.5, base_y),
                Vec2::new(base_x + 1.0, base_y),
                Vec2::new(base_x + 1.0, base_y + 1.0),
                Vec2::new(base_x, base_y + 1.0),
                Vec2::new(base_x, base_y + 0.5),
            ],
            triangles: vec![
                [0, 1, 2],
                [0, 2, 3],
                [0, 3, 4],
            ],
        },
        _ => create_generic_mesh(cell_x, cell_y, [true, true, true, true]), // fallback
    }
}

fn create_edge_mesh(cell_x: f32, cell_y: f32, edge: usize) -> CellMesh {
    let base_x = cell_x;
    let base_y = cell_y;
    
    match edge {
        0 => CellMesh { // bottom edge
            vertices: vec![
                Vec2::new(base_x, base_y),
                Vec2::new(base_x + 1.0, base_y),
                Vec2::new(base_x + 1.0, base_y + 0.5),
                Vec2::new(base_x, base_y + 0.5),
            ],
            triangles: vec![
                [0, 1, 2],
                [0, 2, 3],
            ],
        },
        1 => CellMesh { // right edge
            vertices: vec![
                Vec2::new(base_x + 0.5, base_y),
                Vec2::new(base_x + 1.0, base_y),
                Vec2::new(base_x + 1.0, base_y + 1.0),
                Vec2::new(base_x + 0.5, base_y + 1.0),
            ],
            triangles: vec![
                [0, 1, 2],
                [0, 2, 3],
            ],
        },
        2 => CellMesh { // top edge
            vertices: vec![
                Vec2::new(base_x, base_y + 0.5),
                Vec2::new(base_x + 1.0, base_y + 0.5),
                Vec2::new(base_x + 1.0, base_y + 1.0),
                Vec2::new(base_x, base_y + 1.0),
            ],
            triangles: vec![
                [0, 1, 2],
                [0, 2, 3],
            ],
        },
        3 => CellMesh { // left edge
            vertices: vec![
                Vec2::new(base_x, base_y),
                Vec2::new(base_x + 0.5, base_y),
                Vec2::new(base_x + 0.5, base_y + 1.0),
                Vec2::new(base_x, base_y + 1.0),
            ],
            triangles: vec![
                [0, 1, 2],
                [0, 2, 3],
            ],
        },
        _ => unreachable!(),
    }
}

fn create_inverse_edge_mesh(cell_x: f32, cell_y: f32, _edge: usize) -> CellMesh {
    // Simplified - just create full square for now
    create_generic_mesh(cell_x, cell_y, [true, true, true, true])
}

fn create_diagonal_mesh(cell_x: f32, cell_y: f32, flip: bool) -> CellMesh {
    let base_x = cell_x;
    let base_y = cell_y;
    
    if flip {
        // bottom-right + top-left
        CellMesh {
            vertices: vec![
                Vec2::new(base_x + 0.5, base_y),      // bottom center
                Vec2::new(base_x + 1.0, base_y),      // bottom-right
                Vec2::new(base_x + 1.0, base_y + 0.5), // right center
                Vec2::new(base_x, base_y + 0.5),      // left center
                Vec2::new(base_x, base_y + 1.0),      // top-left
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
                Vec2::new(base_x, base_y),            // bottom-left
                Vec2::new(base_x + 0.5, base_y),      // bottom center
                Vec2::new(base_x, base_y + 0.5),      // left center
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
    
    // Simple approach: create a quad if any corners are filled
    if corners.iter().any(|&c| c) {
        let base_x = cell_x;
        let base_y = cell_y;
        
        vertices = vec![
            Vec2::new(base_x, base_y),
            Vec2::new(base_x + 1.0, base_y),
            Vec2::new(base_x + 1.0, base_y + 1.0),
            Vec2::new(base_x, base_y + 1.0),
        ];
        
        triangles = vec![
            [0, 1, 2],
            [0, 2, 3],
        ];
    }
    
    CellMesh { vertices, triangles }
}