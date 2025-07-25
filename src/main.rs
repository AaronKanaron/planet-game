mod camera;
mod planet;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::input::ButtonInput;
use bevy::window::WindowPlugin;
use bevy::{diagnostic::LogDiagnosticsPlugin, prelude::*};

// use crate::camera::CameraPlugin;
use crate::planet::plugin::PlanetPlugin;
use crate::planet::world::chunk_world::World;
use crate::planet::{meshing::mesh_renderer::VOXEL_SIZE, world::voxel::VoxelType};

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
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
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
        let world_x = (world_pos.x / VOXEL_SIZE) as isize;
        let world_y = (world_pos.y / VOXEL_SIZE) as isize;

        world.set_voxel(world_x as i32, world_y as i32, VoxelType::Air);
        // world.set_voxel((world_x + 1) as i32, (world_y) as i32, VoxelType::Air);
        // world.set_voxel((world_x + 1) as i32, (world_y + 1) as i32, VoxelType::Air);
        // world.set_voxel((world_x) as i32, (world_y + 1) as i32, VoxelType::Air);
        // world.set_voxel((world_x - 1) as i32, (world_y) as i32, VoxelType::Air);
        // world.set_voxel((world_x - 1) as i32, (world_y - 1) as i32, VoxelType::Air);
        // world.set_voxel((world_x) as i32, (world_y - 1) as i32, VoxelType::Air);
        // world.set_voxel((world_x + 1) as i32, (world_y - 1) as i32, VoxelType::Air);
        // world.set_voxel((world_x - 1) as i32, (world_y + 1) as i32, VoxelType::Air);
    }
}
