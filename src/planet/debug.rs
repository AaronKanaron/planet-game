use bevy::prelude::*;

use crate::planet::rendering::culling::ChunkCullingBox;
use crate::planet::world::chunk_world::World;

#[derive(Resource)]
pub struct DebugState {
    // Simplified debug state - removed wireframe and chunk boundaries
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            // Empty - no debug features enabled by default
        }
    }
}

#[derive(Component)]
pub struct DebugText;

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

pub fn update_debug_info(
    chunk_world: Res<World>,
    culling_box: Res<ChunkCullingBox>,
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
            "\n\nLoaded Chunks: {} / {} expected\nCulling Box: {:.1}, {:.1} ({}x{})\nPadding: {:.1}\nChunk Range: ({}, {}) to ({}, {})\n\nControls:\nWASD - Move box\nArrows - Resize box\nB/N - Increase/Decrease padding\nSpace - Toggle culling\nI/O - Zoom in/out",
            chunk_world.loaded_chunk_count(),
            expected_chunks,
            culling_box.center.x,
            culling_box.center.y,
            culling_box.half_extents.x * 2.0,
            culling_box.half_extents.y * 2.0,
            culling_box.padding,
            // culling_box.enabled,
            min_chunk_x,
            min_chunk_y,
            max_chunk_x,
            max_chunk_y
        );
    }
}

/// System to handle debug input (minimal - no features to toggle)
pub fn debug_input_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // No debug features to toggle - kept for future expansion
    if keyboard_input.just_pressed(KeyCode::F1) {
        info!("Debug key F1 pressed - no features available");
    }
}
