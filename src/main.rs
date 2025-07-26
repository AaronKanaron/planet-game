mod camera;
mod planet;
mod utils;

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::WindowPlugin;

use crate::planet::plugin::PlanetPlugin;
use crate::planet::world::chunk_world::World;
use crate::planet::{meshing::mesh_renderer::VOXEL_SIZE, world::voxel::VoxelType};
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
        .add_plugins(DebugPlugin)
        .add_plugins(PlanetPlugin)
        // .add_plugins(CameraPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, handle_input)
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
