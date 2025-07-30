/* Imports */
use bevy::prelude::*;

use crate::planet::{
    meshing::{dual_contouring::DualContouring, render_voxels::VOXEL_SIZE},
    rendering::culling::ChunkCullingBox,
    world::{chunk::CHUNK_SIZE, chunk_world::World},
};

/* Structs */
#[derive(Resource)]
pub struct DebugState {
    pub show_contour_cells: bool,
    pub show_normals: bool,
    pub show_interior_vertices: bool,
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
                "\n\nLoaded Chunks: {} / {} expected\nCulling Box: {:.1}, {:.1} ({}x{})\nPadding: {:.1}\nChunk Range: ({}, {}) to ({}, {})\n\nDebug Visualization:\nF1 - Contour Cells: {}\nF2 - Normals: {}\nF3 - Interior Vertices: {}\n\nControls:\nWASD - Move box\nArrows - Resize box\nB/N - Increase/Decrease padding\nSpace - Toggle culling\nI/O - Zoom in/out",
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
            );
        }
    }

    /// System to handle debug input (minimal - no features to toggle)
    pub fn input_system(
        keyboard_input: Res<ButtonInput<KeyCode>>,
        mut debug_state: ResMut<DebugState>,
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
        {
            return;
        }

        // Render contour cells for all loaded chunks using neighbor-aware detection
        for (&(chunk_x, chunk_y), _chunk) in &world.loaded_chunks {
            let contour_cells = DualContouring::find_contour_cells(&world, chunk_x, chunk_y);
            Self::render_contour_cells(&contour_cells, chunk_x, chunk_y, &mut gizmos, &debug_state);
        }
    }
}
