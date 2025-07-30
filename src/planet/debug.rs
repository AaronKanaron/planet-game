/* Imports */
use bevy::prelude::*;

use crate::planet::{
    meshing::{
        dual_contouring::DualContouring, mesh_renderer::ChunkMesh, render_voxels::VOXEL_SIZE,
        render_voxels::VoxelRenderer,
    },
    rendering::{
        culling::ChunkCullingBox,
        materials::{CoreMaterial, DirtMaterial, GrassMaterial, RockMaterial},
    },
    world::{chunk::CHUNK_SIZE, chunk_world::World, voxel::VoxelType},
};

/* Structs */
#[derive(Resource)]
pub struct DebugState {
    pub show_contour_cells: bool,
    pub show_normals: bool,
    pub show_interior_vertices: bool,
    pub show_contour_lines: bool,
    pub show_voxels: bool,
}

#[derive(Component)]
pub struct DebugText;
pub struct DebugPlugin;

/* Implementations */
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::setup).add_systems(
            Update,
            (
                Self::update_info,
                Self::input_system,
                Self::render_contour_gizmos_system,
                Self::render_voxels_system,
                Self::cleanup_unloaded_chunks,
            ),
        );
    }
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            show_contour_cells: false,
            show_normals: false,
            show_interior_vertices: false,
            show_contour_lines: false,
            show_voxels: true, // Show voxels by default
        }
    }
}

impl DebugPlugin {
    pub fn setup(mut commands: Commands) {
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

    /// System to setup debug UI
    pub fn update_info(
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
                "\n\nLoaded Chunks: {} / {} expected\nCulling Box: {:.1}, {:.1} ({}x{})\nPadding: {:.1}\nChunk Range: ({}, {}) to ({}, {})\n\nDebug Visualization:\nF1 - Contour Cells: {}\nF2 - Normals: {}\nF3 - Interior Vertices: {}\nF4 - Contour Lines: {}\nF5 - Voxels: {}\n\nControls:\nWASD - Move box\nArrows - Resize box\nB/N - Increase/Decrease padding\nSpace - Toggle culling\nI/O - Zoom in/out",
                chunk_world.loaded_chunk_count(),
                expected_chunks,
                culling_box.center.x,
                culling_box.center.y,
                culling_box.half_extents.x * 2.0,
                culling_box.half_extents.y * 2.0,
                culling_box.padding,
                min_chunk_x,
                min_chunk_y,
                max_chunk_x,
                max_chunk_y,
                if debug_state.show_contour_cells {
                    "ON"
                } else {
                    "OFF"
                },
                if debug_state.show_normals {
                    "ON"
                } else {
                    "OFF"
                },
                if debug_state.show_interior_vertices {
                    "ON"
                } else {
                    "OFF"
                },
                if debug_state.show_contour_lines {
                    "ON"
                } else {
                    "OFF"
                },
                if debug_state.show_voxels { "ON" } else { "OFF" },
            );
        }
    }

    /// System to handle debug input (minimal - no features to toggle)
    pub fn input_system(
        keyboard_input: Res<ButtonInput<KeyCode>>,
        mut debug_state: ResMut<DebugState>,
        mut world: ResMut<World>,
    ) {
        if keyboard_input.just_pressed(KeyCode::F1) {
            debug_state.show_contour_cells = !debug_state.show_contour_cells;
            info!("Contour cells: {}", debug_state.show_contour_cells);
        }

        if keyboard_input.just_pressed(KeyCode::F2) {
            debug_state.show_normals = !debug_state.show_normals;
            info!("Normals: {}", debug_state.show_normals);
        }

        if keyboard_input.just_pressed(KeyCode::F3) {
            debug_state.show_interior_vertices = !debug_state.show_interior_vertices;
            info!("Interior vertices: {}", debug_state.show_interior_vertices);
        }

        if keyboard_input.just_pressed(KeyCode::F4) {
            debug_state.show_contour_lines = !debug_state.show_contour_lines;
            info!("Contour lines: {}", debug_state.show_contour_lines);
        }

        if keyboard_input.just_pressed(KeyCode::F5) {
            let was_enabled = debug_state.show_voxels;
            debug_state.show_voxels = !debug_state.show_voxels;
            info!("Voxels: {}", debug_state.show_voxels);

            // If voxels were just enabled, force update all chunks
            if !was_enabled && debug_state.show_voxels {
                // Mark all loaded chunks as dirty
                let chunk_positions: Vec<(i32, i32)> =
                    world.loaded_chunks.keys().cloned().collect();
                for (chunk_x, chunk_y) in chunk_positions {
                    world.mark_chunk_dirty(chunk_x, chunk_y);
                }
                info!(
                    "Marked {} chunks dirty for voxel rendering",
                    world.loaded_chunks.len()
                );
            }
        }
    }

    /// Render contour cells with debug toggles
    pub fn render_contour_cells(
        contour_cells: &[crate::planet::meshing::dual_contouring::ContourCell],
        chunk_x: i32,
        chunk_y: i32,
        gizmos: &mut Gizmos,
        debug_state: &DebugState,
    ) {
        for cell in contour_cells {
            let world_x = (chunk_x * CHUNK_SIZE as i32 + cell.x as i32) as f32 * VOXEL_SIZE
                + VOXEL_SIZE * 0.5;
            let world_y = (chunk_y * CHUNK_SIZE as i32 + cell.y as i32) as f32 * VOXEL_SIZE
                + VOXEL_SIZE * 0.5;

            // Draw cell boundary and corner indicators (F1 toggle)
            if debug_state.show_contour_cells {
                let half_size = VOXEL_SIZE * 0.4;

                // Draw cell boundary (green square)
                let corners = [
                    Vec2::new(world_x - half_size, world_y - half_size),
                    Vec2::new(world_x + half_size, world_y - half_size),
                    Vec2::new(world_x + half_size, world_y + half_size),
                    Vec2::new(world_x - half_size, world_y + half_size),
                ];

                for i in 0..4 {
                    let start = corners[i];
                    let end = corners[(i + 1) % 4];
                    gizmos.line_2d(start, end, Color::srgb(0.0, 1.0, 0.0));
                }

                // Draw corner indicators (yellow/gray circles)
                let corner_size = VOXEL_SIZE * 0.1;
                let corner_positions = [
                    Vec2::new(world_x - half_size, world_y - half_size),
                    Vec2::new(world_x + half_size, world_y - half_size),
                    Vec2::new(world_x - half_size, world_y + half_size),
                    Vec2::new(world_x + half_size, world_y + half_size),
                ];

                for (i, &corner_pos) in corner_positions.iter().enumerate() {
                    if cell.corner_values[i] {
                        gizmos.circle_2d(corner_pos, corner_size, Color::srgb(1.0, 1.0, 0.0)); // Yellow for solid
                    } else {
                        gizmos.circle_2d(corner_pos, corner_size * 0.5, Color::srgb(0.5, 0.5, 0.5)); // Gray for air
                    }
                }
            }

            // Calculate absolute world position for interior vertex
            let chunk_world_x = chunk_x as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE;
            let chunk_world_y = chunk_y as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE;
            let cell_world_x = chunk_world_x + cell.x as f32 * VOXEL_SIZE;
            let cell_world_y = chunk_world_y + cell.y as f32 * VOXEL_SIZE;

            // Convert from local coordinates (0-1) to world coordinates
            let absolute_vertex_pos = Vec2::new(
                cell_world_x + cell.interior_vertex.x * VOXEL_SIZE,
                cell_world_y + cell.interior_vertex.y * VOXEL_SIZE,
            );

            // Draw interior vertex (F3 toggle)
            if debug_state.show_interior_vertices {
                let vertex_size = VOXEL_SIZE * 0.15;
                gizmos.circle_2d(absolute_vertex_pos, vertex_size, Color::srgb(1.0, 0.0, 0.0)); // Red circle
            }

            // Draw normal vector (F2 toggle)
            if debug_state.show_normals {
                let normal_length = VOXEL_SIZE * 0.8;
                let normal_end = absolute_vertex_pos + cell.normal * normal_length;
                gizmos.line_2d(absolute_vertex_pos, normal_end, Color::srgb(1.0, 0.0, 0.0)); // Red line

                // Draw small arrowhead for normal direction
                let arrow_size = VOXEL_SIZE * 0.1;
                let perpendicular = Vec2::new(-cell.normal.y, cell.normal.x) * arrow_size;
                let arrow_base = normal_end - cell.normal * arrow_size;
                gizmos.line_2d(
                    normal_end,
                    arrow_base + perpendicular,
                    Color::srgb(1.0, 0.0, 0.0),
                );
                gizmos.line_2d(
                    normal_end,
                    arrow_base - perpendicular,
                    Color::srgb(1.0, 0.0, 0.0),
                );
            }
        }

        // Draw contour lines connecting adjacent vertices (F4 toggle)
        if debug_state.show_contour_lines {
            Self::render_contour_lines(contour_cells, chunk_x, chunk_y, gizmos);
        }
    }

    /// Render contour lines connecting adjacent interior vertices
    fn render_contour_lines(
        contour_cells: &[crate::planet::meshing::dual_contouring::ContourCell],
        chunk_x: i32,
        chunk_y: i32,
        gizmos: &mut Gizmos,
    ) {
        // Create a map of cell positions to their absolute vertex positions for quick lookup
        let mut cell_vertices = std::collections::HashMap::new();

        for cell in contour_cells {
            let chunk_world_x = chunk_x as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE;
            let chunk_world_y = chunk_y as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE;
            let cell_world_x = chunk_world_x + cell.x as f32 * VOXEL_SIZE;
            let cell_world_y = chunk_world_y + cell.y as f32 * VOXEL_SIZE;

            let absolute_vertex_pos = Vec2::new(
                cell_world_x + cell.interior_vertex.x * VOXEL_SIZE,
                cell_world_y + cell.interior_vertex.y * VOXEL_SIZE,
            );

            cell_vertices.insert((cell.x, cell.y), absolute_vertex_pos);
        }

        // Connect adjacent vertices that share an edge
        for cell in contour_cells {
            let current_pos = cell_vertices.get(&(cell.x, cell.y)).unwrap();

            // Check all 4 adjacent cells (right, up, left, down)
            let adjacents = [
                (cell.x + 1, cell.y),             // right
                (cell.x, cell.y + 1),             // up
                (cell.x.wrapping_sub(1), cell.y), // left (with underflow protection)
                (cell.x, cell.y.wrapping_sub(1)), // down (with underflow protection)
            ];

            for &(adj_x, adj_y) in &adjacents {
                // Only draw lines to right and up neighbors to avoid drawing each line twice
                if (adj_x == cell.x + 1 && adj_y == cell.y)
                    || (adj_x == cell.x && adj_y == cell.y + 1)
                {
                    if let Some(adj_pos) = cell_vertices.get(&(adj_x, adj_y)) {
                        // Check if the edge between these cells should have a contour line
                        // This happens when the cells share a common edge with a sign change
                        if Self::should_connect_vertices(cell, adj_x, adj_y, contour_cells) {
                            gizmos.line_2d(*current_pos, *adj_pos, Color::srgb(0.0, 0.8, 1.0)); // Cyan contour lines
                        }
                    }
                }
            }
        }
    }

    /// Check if two adjacent cells should be connected with a contour line
    fn should_connect_vertices(
        cell: &crate::planet::meshing::dual_contouring::ContourCell,
        adj_x: usize,
        adj_y: usize,
        all_cells: &[crate::planet::meshing::dual_contouring::ContourCell],
    ) -> bool {
        // Find the adjacent cell
        if let Some(_adj_cell) = all_cells.iter().find(|c| c.x == adj_x && c.y == adj_y) {
            // Two contour cells should be connected if they share a common edge
            // and both have interior vertices (which they do, since they're both contour cells)

            // Determine which edge they share
            if adj_x == cell.x + 1 && adj_y == cell.y {
                // Adjacent to the right - they share the right edge of current cell
                // Connect if both cells exist (which they do)
                true
            } else if adj_x == cell.x && adj_y == cell.y + 1 {
                // Adjacent upward - they share the top edge of current cell
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// System to render contour gizmos with debug toggles
    pub fn render_contour_gizmos_system(
        world: Res<World>,
        debug_state: Res<DebugState>,
        mut gizmos: Gizmos,
    ) {
        // Only render if any debug features are enabled
        if !debug_state.show_contour_cells
            && !debug_state.show_normals
            && !debug_state.show_interior_vertices
            && !debug_state.show_contour_lines
        {
            return;
        }

        // Render contour cells for all loaded chunks using neighbor-aware detection
        for (&(chunk_x, chunk_y), _chunk) in &world.loaded_chunks {
            let contour_cells = DualContouring::find_contour_cells(&world, chunk_x, chunk_y);
            Self::render_contour_cells(&contour_cells, chunk_x, chunk_y, &mut gizmos, &debug_state);
        }
    }

    /// System to render voxels with debug toggle
    pub fn render_voxels_system(
        mut commands: Commands,
        mut world: ResMut<World>,
        debug_state: Res<DebugState>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut rock_materials: ResMut<Assets<RockMaterial>>,
        mut dirt_materials: ResMut<Assets<DirtMaterial>>,
        mut grass_materials: ResMut<Assets<GrassMaterial>>,
        mut core_materials: ResMut<Assets<CoreMaterial>>,
        existing_chunks: Query<(Entity, &ChunkMesh)>,
    ) {
        if !debug_state.show_voxels {
            // If voxels are disabled, despawn all existing voxel meshes
            for (entity, _) in existing_chunks.iter() {
                commands.entity(entity).despawn();
            }
            return;
        }

        let dirty_chunks = world.get_dirty_chunks();
        if dirty_chunks.is_empty() {
            return;
        }

        // Despawn dirty chunks
        for (entity, chunk_mesh) in existing_chunks.iter() {
            if dirty_chunks.contains(&(chunk_mesh.chunk_x, chunk_mesh.chunk_y)) {
                commands.entity(entity).despawn();
            }
        }

        // Regenerate meshes for dirty chunks only
        for &(chunk_x, chunk_y) in &dirty_chunks {
            // Skip if chunk is no longer loaded (might have been unloaded)
            if !world.is_chunk_loaded(chunk_x, chunk_y) {
                continue;
            }

            let chunk = world.loaded_chunks.get(&(chunk_x, chunk_y)).unwrap();

            // Create separate meshes for each material type
            for &material_type in &[
                VoxelType::Rock,
                VoxelType::Dirt,
                VoxelType::Grass,
                VoxelType::Core,
            ] {
                // Delegate to the voxel renderer
                VoxelRenderer::render_voxels(
                    chunk,
                    chunk_x,
                    chunk_y,
                    material_type,
                    &mut commands,
                    &mut meshes,
                    &mut rock_materials,
                    &mut dirt_materials,
                    &mut grass_materials,
                    &mut core_materials,
                );
            }
            // Remove dirty flag
            world.mark_chunk_clean(chunk_x, chunk_y);
        }
    }

    /// System to cleanup unloaded chunks
    pub fn cleanup_unloaded_chunks(
        commands: Commands,
        world: Res<World>,
        existing_chunks: Query<(Entity, &ChunkMesh)>,
    ) {
        VoxelRenderer::cleanup_unloaded_chunks(commands, world, existing_chunks);
    }
}
