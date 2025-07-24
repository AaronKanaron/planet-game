use bevy::prelude::*;
use crate::planet::culling::ChunkCullingBox;
use crate::planet::mesh::chunk_world::ChunkWorld;

/// System that handles chunk loading and unloading based on the culling box
pub fn chunk_culling_system(
    mut chunk_world: ResMut<ChunkWorld>,
    culling_box: Res<ChunkCullingBox>,
) {
    if !culling_box.enabled {
        return;
    }

    // Only process when culling box changes or we need to check chunks
    let should_process = culling_box.is_changed() || chunk_world.loaded_chunk_count() == 0;
    
    if !should_process {
        return;
    }

    // Load chunks within the bounding box
    let loaded_chunks = chunk_world.load_chunks_in_box(&culling_box);

    // Unload chunks outside the bounding box
    let unloaded_chunks = chunk_world.unload_chunks_outside_box(&culling_box);
    
    if !unloaded_chunks.is_empty() {
        info!("Unloaded {} chunks outside culling box", unloaded_chunks.len());
    }
    
    if !loaded_chunks.is_empty() {
        info!("Loaded {} new chunks", loaded_chunks.len());
    }

    // Only mark newly loaded chunks as dirty, not all chunks
    for &chunk_pos in &loaded_chunks {
        chunk_world.mark_chunk_dirty(chunk_pos.0, chunk_pos.1);
        
        // Also mark neighboring chunks as dirty to ensure proper edge connectivity
        let neighbors = [
            (chunk_pos.0 - 1, chunk_pos.1),     // left
            (chunk_pos.0 + 1, chunk_pos.1),     // right
            (chunk_pos.0, chunk_pos.1 - 1),     // bottom
            (chunk_pos.0, chunk_pos.1 + 1),     // top
        ];
        
        for &(nx, ny) in &neighbors {
            chunk_world.mark_chunk_dirty(nx, ny);
        }
    }
}

/// System that draws the culling box gizmo
pub fn draw_culling_box_gizmo(
    mut gizmos: Gizmos,
    culling_box: Res<ChunkCullingBox>,
) {
    if !culling_box.enabled {
        return;
    }

    let min = culling_box.center - culling_box.half_extents;
    let max = culling_box.center + culling_box.half_extents;

    // Draw the bounding box outline
    gizmos.rect_2d(
        culling_box.center,
        culling_box.half_extents * 2.0, // size
        Color::linear_rgb(1.0, 1.0, 0.0), // yellow
    );

    // Draw corner markers
    let corner_size = 10.0;
    let corners = [
        Vec2::new(min.x, min.y),
        Vec2::new(max.x, min.y),
        Vec2::new(max.x, max.y),
        Vec2::new(min.x, max.y),
    ];

    for corner in corners {
        gizmos.rect_2d(
            corner,
            Vec2::splat(corner_size),
            Color::linear_rgb(1.0, 0.5, 0.0), // orange
        );
    }

    // Draw center marker
    gizmos.circle_2d(
        culling_box.center,
        5.0,
        Color::linear_rgb(1.0, 0.0, 0.0), // red
    );
}

/// System that handles input for moving and resizing the culling box
pub fn culling_box_input_system(
    mut culling_box: ResMut<ChunkCullingBox>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let move_speed = 100.0; // units per second
    let resize_speed = 50.0; // units per second
    
    let delta_time = time.delta_secs();

    // Movement controls (WASD) - Fixed Y-axis direction
    if keyboard_input.pressed(KeyCode::KeyW) {
        culling_box.center.y += move_speed * delta_time;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        culling_box.center.y -= move_speed * delta_time;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        culling_box.center.x -= move_speed * delta_time;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        culling_box.center.x += move_speed * delta_time;
    }

    // Resize controls (Arrow keys)
    if keyboard_input.pressed(KeyCode::ArrowUp) {
        culling_box.half_extents.y += resize_speed * delta_time;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        culling_box.half_extents.y = (culling_box.half_extents.y - resize_speed * delta_time).max(10.0);
    }
    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        culling_box.half_extents.x = (culling_box.half_extents.x - resize_speed * delta_time).max(10.0);
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        culling_box.half_extents.x += resize_speed * delta_time;
    }

    // Toggle culling box (Space key)
    if keyboard_input.just_pressed(KeyCode::Space) {
        culling_box.enabled = !culling_box.enabled;
        info!("Chunk culling {}", if culling_box.enabled { "enabled" } else { "disabled" });
    }
}
