mod camera;
mod planet;
mod tiles;
mod utils;

use bevy::input::{ButtonInput, mouse::MouseWheel};
use bevy::prelude::*;
use bevy::window::WindowPlugin;

use crate::planet::plugin::PlanetPlugin;
use crate::planet::rendering::culling::ChunkCullingBox;
use crate::planet::world::chunk_world::World;
use crate::planet::world::voxel::{VOXEL_SIZE, VoxelType};
use crate::tiles::TilePlugin;
use crate::tiles::data::GenericTileData;
use crate::tiles::preview::TilePreview;
use crate::tiles::variants::loudspeaker::Loudspeaker;
use crate::utils::debug::plugin::DebugPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Planet Game 2".to_string(),
                        resolution: (800.0, 600.0).into(),
                        ..default()
                    }),
                    ..Default::default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins((DebugPlugin, PlanetPlugin, TilePlugin))
        // .add_plugins(CameraPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_input,
                follow_culling_center,
                handle_zoom,
                debug_keypress,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..Default::default()
        },
        Transform::default(),
        GlobalTransform::default(),
    ));
}

fn debug_keypress(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    preview: Query<Entity, With<TilePreview>>,
    mut commands: Commands,
) {
    if keyboard_input.just_pressed(KeyCode::KeyK) {
        commands.spawn(Loudspeaker::new(GenericTileData::new(0, true)));
    }
    if keyboard_input.just_pressed(KeyCode::KeyL) {
        if let Ok(preview) = preview.single() {
            commands.entity(preview).remove::<TilePreview>();
        }
    }
}

fn handle_input(
    mut world: ResMut<World>,
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
        // Convert world position to voxel grid coordinates
        let voxel_x = (world_pos.x / VOXEL_SIZE) as i32;
        let voxel_y = (world_pos.y / VOXEL_SIZE) as i32;

        // Due to dual contouring, the visual mesh at position (x, y) is generated from
        // the cell configuration of voxels at (x, y), (x+1, y), (x+1, y+1), and (x, y+1).
        // To properly remove the visual block we're clicking on, we need to set all
        // 4 corner voxels of that cell to Air.
        world.set_voxel(voxel_x, voxel_y, VoxelType::Air);
        world.set_voxel(voxel_x + 1, voxel_y, VoxelType::Air);
        world.set_voxel(voxel_x + 1, voxel_y + 1, VoxelType::Air);
        world.set_voxel(voxel_x, voxel_y + 1, VoxelType::Air);
    }
}

//camera follow
fn follow_culling_center(
    mut camera_transform: Single<&mut Transform, With<Camera>>,
    culling_box: Res<ChunkCullingBox>,
) {
    if culling_box.is_changed() {
        camera_transform.translation.x = culling_box.center.x;
        camera_transform.translation.y = culling_box.center.y;
    }
}

fn handle_zoom(
    mut scroll_events: EventReader<MouseWheel>,
    mut projection: Single<&mut Projection, With<Camera>>,
) {
    let Projection::Orthographic(projection) = &mut **projection else {
        return;
    };

    let zoom_factor = 1.1;
    let min_scale = 0.02;
    let max_scale = 100.0;

    for event in scroll_events.read() {
        if event.y > 0.0 {
            // Scroll up: zoom in (decrease scale)
            projection.scale = (projection.scale / zoom_factor).max(min_scale);
        } else if event.y < 0.0 {
            // Scroll down: zoom out (increase scale)
            projection.scale = (projection.scale * zoom_factor).min(max_scale);
        }
    }
}
