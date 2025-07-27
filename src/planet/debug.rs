use crate::planet::rendering::culling::ChunkCullingBox;
use crate::planet::world::chunk_world::World;
use crate::planet::meshing::mesh_renderer::{ChunkMesh, VOXEL_SIZE};
use bevy::prelude::*;

#[derive(Resource)]
pub struct DebugState {
    pub wireframe_enabled: bool,
    pub chunk_boundaries_enabled: bool,
    pub total_triangles: usize,
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            wireframe_enabled: false,
            chunk_boundaries_enabled: false,
            total_triangles: 0,
        }
    }
}

#[derive(Component)]
pub struct DebugText;

#[derive(Component)]
pub struct WireframeMesh;

#[derive(Component)]
pub struct ChunkBoundary;

#[derive(Component)]
pub struct TriangleCount(pub usize);

/// System to setup debug UI
pub fn setup_debug_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("Debug Info"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        DebugText,
    ));
}

/// System to update debug information
pub fn update_debug_info(
    chunk_world: Res<World>,
    culling_box: Res<ChunkCullingBox>,
    debug_state: Res<DebugState>,
    mut query: Query<&mut Text, With<DebugText>>,
) {
    if let Ok(mut text) = query.single_mut() {
        let chunk_size = World::chunk_size();
        let (min_chunk_x, min_chunk_y, max_chunk_x, max_chunk_y) =
            culling_box.get_chunk_bounds(chunk_size);

        let expected_chunks = if culling_box.enabled {
            let mut count = 0;
            for chunk_x in min_chunk_x..=max_chunk_x {
                for chunk_y in min_chunk_y..=max_chunk_y {
                    if culling_box.contains_chunk(chunk_x, chunk_y, chunk_size) {
                        count += 1;
                    }
                }
            }
            count
        } else {
            chunk_world.loaded_chunk_count()
        };

        **text = format!(
            "\n\nLoaded Chunks: {} / {} expected\nCulling Box: {:.1}, {:.1} ({}x{})\nPadding: {:.1}\nEnabled: {}\nChunk Range: ({}, {}) to ({}, {})\nTriangles: {}\nWireframe: {}\nChunk Boundaries: {}\n\nControls:\nWASD - Move box\nArrows - Resize box\nB/N - Increase/Decrease padding\nSpace - Toggle culling\nF1 - Toggle wireframe\nF2 - Toggle chunk boundaries",
            chunk_world.loaded_chunk_count(),
            expected_chunks,
            culling_box.center.x,
            culling_box.center.y,
            culling_box.half_extents.x * 2.0,
            culling_box.half_extents.y * 2.0,
            culling_box.padding,
            culling_box.enabled,
            min_chunk_x,
            min_chunk_y,
            max_chunk_x,
            max_chunk_y,
            debug_state.total_triangles,
            if debug_state.wireframe_enabled { "ON" } else { "OFF" },
            if debug_state.chunk_boundaries_enabled { "ON" } else { "OFF" }
        );
    }
}

/// System to handle debug input
pub fn debug_input_system(
    mut debug_state: ResMut<DebugState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // Toggle wireframe with F1
    if keyboard_input.just_pressed(KeyCode::F1) {
        debug_state.wireframe_enabled = !debug_state.wireframe_enabled;
        info!("Wireframe {}", if debug_state.wireframe_enabled { "enabled" } else { "disabled" });
    }

    // Toggle chunk boundaries with F2
    if keyboard_input.just_pressed(KeyCode::F2) {
        debug_state.chunk_boundaries_enabled = !debug_state.chunk_boundaries_enabled;
        info!("Chunk boundaries {}", if debug_state.chunk_boundaries_enabled { "enabled" } else { "disabled" });
    }
}

/// System to toggle wireframe mesh visibility
pub fn toggle_wireframe_visibility(
    debug_state: Res<DebugState>,
    mut all_wireframes: Query<&mut Visibility, With<WireframeMesh>>,
) {
    if debug_state.is_changed() {
        for mut visibility in all_wireframes.iter_mut() {
            *visibility = if debug_state.wireframe_enabled {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// System to toggle chunk boundary visibility
pub fn toggle_chunk_boundary_visibility(
    debug_state: Res<DebugState>,
    mut boundary_query: Query<&mut Visibility, With<ChunkBoundary>>,
) {
    if debug_state.is_changed() {
        for mut visibility in boundary_query.iter_mut() {
            *visibility = if debug_state.chunk_boundaries_enabled {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// System to recalculate triangle count from triangle count components
pub fn update_triangle_count(
    chunk_world: Res<World>,
    mut debug_state: ResMut<DebugState>,
    triangle_counts: Query<&TriangleCount, With<ChunkMesh>>,
) {
    // Recalculate if chunks have changed or if we don't have a count yet
    if chunk_world.is_changed() || debug_state.total_triangles == 0 {
        let total_triangles: usize = triangle_counts.iter().map(|tc| tc.0).sum();
        
        if debug_state.total_triangles != total_triangles {
            info!("Triangle count updated: {} triangles from {} chunks", total_triangles, triangle_counts.iter().count());
            debug_state.total_triangles = total_triangles;
        }
    }
}

/// System to create chunk boundary wireframes
pub fn create_chunk_boundaries(
    mut commands: Commands,
    chunk_world: Res<World>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    existing_boundaries: Query<(Entity, &ChunkBoundary)>,
    debug_state: Res<DebugState>,
) {
    // Only update if chunks changed or debug state changed
    if !chunk_world.is_changed() && !debug_state.is_changed() {
        return;
    }

    // Remove existing boundaries
    for (entity, _) in existing_boundaries.iter() {
        commands.entity(entity).despawn();
    }

    // Create boundary wireframes for all loaded chunks
    for (chunk_x, chunk_y) in chunk_world.loaded_chunks.keys() {
        let chunk_size = World::chunk_size() as f32;
        let world_x = *chunk_x as f32 * chunk_size * VOXEL_SIZE;
        let world_y = *chunk_y as f32 * chunk_size * VOXEL_SIZE;
        let size = chunk_size * VOXEL_SIZE;

        // Create vertices for chunk boundary
        let vertices = vec![
            [world_x, world_y, 0.0],                  // Bottom-left
            [world_x + size, world_y, 0.0],           // Bottom-right
            [world_x + size, world_y + size, 0.0],    // Top-right
            [world_x, world_y + size, 0.0],           // Top-left
        ];

        // Create indices for wireframe lines
        let indices = vec![
            0, 1,  // Bottom edge
            1, 2,  // Right edge
            2, 3,  // Top edge
            3, 0,  // Left edge
        ];

        let mut boundary_mesh = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::LineList,
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        
        boundary_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        boundary_mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

        commands.spawn((
            Mesh2d(meshes.add(boundary_mesh)),
            MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(1.0, 1.0, 0.0)))), // Yellow boundaries
            Transform::from_xyz(0.0, 0.0, 0.2), // In front of everything
            ChunkBoundary,
            if debug_state.chunk_boundaries_enabled {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        ));
    }
}
