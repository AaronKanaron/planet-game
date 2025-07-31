use crate::planet::world::chunk_world::World;
use bevy::prelude::*;

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
                world.find_closest_normal(cursor_pos, f32::INFINITY)
            {
                follower.current_normal = closest_normal;
                follower.target_position = closest_position;
            } else {
                // If no normal found, just use the raw cursor position
                follower.target_position = cursor_pos;
            }
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
