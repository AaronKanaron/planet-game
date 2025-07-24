mod planet;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::input::ButtonInput;
use bevy::window::WindowPlugin;
use bevy::{diagnostic::LogDiagnosticsPlugin, prelude::*};
use crate::planet::mesh::chunk_world::ChunkWorld;
use crate::planet::mesh::VoxelType;
use crate::planet::{mesh::VOXEL_SIZE, plugin::PlanetPlugin};

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

        .add_plugins(PlanetPlugin)

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
        GlobalTransform::default()
    ));
}

fn handle_input(
    mut world: ResMut<ChunkWorld>,
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
        let world_x = ((world_pos.x + 400.0) / VOXEL_SIZE) as isize;
        let world_y = ((300.0 - world_pos.y) / VOXEL_SIZE) as isize;

        if world_x >= 0 && world_y >= 0 {
            world.set_voxel(world_x as i32, world_y as i32, VoxelType::Air);
            world.set_voxel((world_x + 1) as i32, (world_y) as i32, VoxelType::Air);
            world.set_voxel(
                (world_x + 1) as i32,
                (world_y + 1) as i32,
                VoxelType::Air,
            );
            world.set_voxel((world_x) as i32, (world_y + 1) as i32, VoxelType::Air);
            world.set_voxel((world_x - 1) as i32, (world_y) as i32, VoxelType::Air);
            world.set_voxel(
                (world_x - 1) as i32,
                (world_y - 1) as i32,
                VoxelType::Air,
            );
        }
    }
}
