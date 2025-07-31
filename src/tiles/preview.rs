use bevy::prelude::*;

use crate::planet::{
    meshing::dual_contouring::DualContouring,
    world::{chunk::CHUNK_SIZE, chunk_world::World, voxel::VOXEL_SIZE},
};

/// This component should only exist once or not at all, and is
/// attached to the tile which is currently being previewed for
/// placement
#[derive(Component)]
pub struct TilePreview {
    pub current_normal: Vec2,
    pub target_position: Vec2,
    pub snap_distance: f32,
    pub raw_cursor_position: Vec2,
}

/// Keeps track of the current state of tile previewing
#[derive(Resource, Default)]
pub struct PreviewState {
    pub active_preview: Option<Entity>,
}

impl TilePreview {
    pub fn cursor_world_position_system(
        windows: Query<&Window>,
        camera_q: Query<(&Camera, &GlobalTransform)>,
        mut follower_query: Query<&mut TilePreview>,
    ) {
        let window = match windows.single() {
            Ok(w) => w,
            Err(_) => return,
        };

        let Ok((camera, camera_transform)) = camera_q.single() else {
            return;
        };

        if let Some(cursor_position) = window.cursor_position() {
            if let Ok(world_position) =
                camera.viewport_to_world_2d(camera_transform, cursor_position)
            {
                for mut follower in follower_query.iter_mut() {
                    follower.raw_cursor_position = world_position;
                }
            }
        }
    }

    pub fn snap_to_closest_normal_system(
        mut world: ResMut<World>,
        mut follower_query: Query<&mut TilePreview>,
    ) {
        for mut follower in follower_query.iter_mut() {
            let cursor_pos = follower.raw_cursor_position;

            if let Some((closest_normal, closest_position)) =
                Self::find_closest_normal(&mut world, cursor_pos, f32::INFINITY)
            {
                follower.current_normal = closest_normal;
                follower.target_position = closest_position;
            } else {
                // If no normal found, just use the raw cursor position
                follower.target_position = cursor_pos;
            }
        }
    }

    fn find_closest_normal(
        world: &mut World,
        cursor_pos: Vec2,
        max_distance: f32,
    ) -> Option<(Vec2, Vec2)> {
        let mut closest_distance = f32::INFINITY;
        let mut closest_normal = None;
        let mut closest_position = None;

        // Calculate which chunks to check based on cursor position and max distance
        let chunk_size_world = CHUNK_SIZE as f32 * VOXEL_SIZE;
        let (min_chunk_x, max_chunk_x, min_chunk_y, max_chunk_y) = if max_distance.is_infinite() {
            // If infinite distance, check all loaded chunks
            let mut min_x = i32::MAX;
            let mut max_x = i32::MIN;
            let mut min_y = i32::MAX;
            let mut max_y = i32::MIN;

            for &(chunk_x, chunk_y) in world.loaded_chunks.keys() {
                min_x = min_x.min(chunk_x);
                max_x = max_x.max(chunk_x);
                min_y = min_y.min(chunk_y);
                max_y = max_y.max(chunk_y);
            }

            if min_x == i32::MAX {
                // No loaded chunks
                return None;
            }

            (min_x, max_x, min_y, max_y)
        } else {
            (
                ((cursor_pos.x - max_distance) / chunk_size_world).floor() as i32,
                ((cursor_pos.x + max_distance) / chunk_size_world).ceil() as i32,
                ((cursor_pos.y - max_distance) / chunk_size_world).floor() as i32,
                ((cursor_pos.y + max_distance) / chunk_size_world).ceil() as i32,
            )
        };

        // Check all nearby chunks for preview points
        for chunk_x in min_chunk_x..=max_chunk_x {
            for chunk_y in min_chunk_y..=max_chunk_y {
                if world.loaded_chunks.contains_key(&(chunk_x, chunk_y)) {
                    let preview_points =
                        DualContouring::get_cached_preview_points(world, chunk_x, chunk_y);

                    for (world_pos, normal) in preview_points {
                        let distance = cursor_pos.distance(world_pos);

                        if distance < closest_distance
                            && (max_distance.is_infinite() || distance <= max_distance)
                        {
                            closest_distance = distance;
                            closest_normal = Some(normal);
                            closest_position = Some(world_pos);
                        }
                    }
                }
            }
        }

        if let (Some(normal), Some(position)) = (closest_normal, closest_position) {
            Some((normal, position))
        } else {
            None
        }
    }

    pub fn update_follower_transform_system(
        mut follower_query: Query<(&TilePreview, &mut Transform)>,
    ) {
        for (follower, mut transform) in follower_query.iter_mut() {
            // Update position
            transform.translation = follower.target_position.extend(0.0);

            // Update rotation to align with normal
            let angle = follower.current_normal.y.atan2(follower.current_normal.x)
                - std::f32::consts::FRAC_PI_2;
            transform.rotation = Quat::from_rotation_z(angle);
        }
    }
}

impl Default for TilePreview {
    fn default() -> Self {
        Self {
            current_normal: Vec2::Y,
            target_position: Vec2::ZERO,
            snap_distance: 0.0,
            raw_cursor_position: Vec2::ZERO,
        }
    }
}
pub struct TilePreviewPlugin;
impl Plugin for TilePreviewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PreviewState>().add_systems(
            Update,
            (
                TilePreview::cursor_world_position_system,
                TilePreview::snap_to_closest_normal_system,
                TilePreview::update_follower_transform_system,
            ),
        );
    }
}
