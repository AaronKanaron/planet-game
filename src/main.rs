use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::input::ButtonInput;
use bevy::window::WindowPlugin;
use bevy::{diagnostic::LogDiagnosticsPlugin, prelude::*};

use crate::planet::plugin::PlanetPlugin;

mod planet;

#[derive(Clone, Copy, PartialEq)]
enum VoxelType {
    Air,
    Rock,
    Dirt,
}

#[derive(Resource)]
struct VoxelWorld {
    width: usize,
    height: usize,
    voxels: Vec<VoxelType>,
}

impl VoxelWorld {
    fn new(width: usize, height: usize) -> Self {
        let voxels = vec![VoxelType::Air; width * height];
        Self {
            width,
            height,
            voxels,
        }
    }

    fn get_voxel(&self, x: usize, y: usize) -> VoxelType {
        if x < self.width && y < self.height{
            self.voxels[y * self.width + x]
        } else {
            VoxelType::Air
        }
    }

    fn set_voxel(&mut self, x: usize, y: usize, voxel_type: VoxelType) {
        if x < self.width && y < self.height {
            self.voxels[y * self.width + x] = voxel_type;
        }
    }    
}

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
    commands.spawn(Camera2d::default());
}





fn handle_input(
    mut world: ResMut<VoxelWorld>,
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
        let world_x = ((world_pos.x + 400.0) / 8.0) as isize;
        let world_y = ((300.0 - world_pos.y) / 8.0) as isize;

        if world_x >= 0 && world_y >= 0 {
            world.set_voxel(world_x as usize, world_y as usize, VoxelType::Air);
            world.set_voxel((world_x + 1) as usize, (world_y) as usize, VoxelType::Air);
            world.set_voxel(
                (world_x + 1) as usize,
                (world_y + 1) as usize,
                VoxelType::Air,
            );
            world.set_voxel((world_x) as usize, (world_y + 1) as usize, VoxelType::Air);
            world.set_voxel((world_x - 1) as usize, (world_y) as usize, VoxelType::Air);
            world.set_voxel(
                (world_x - 1) as usize,
                (world_y - 1) as usize,
                VoxelType::Air,
            );
            world.set_voxel((world_x) as usize, (world_y - 1) as usize, VoxelType::Air);
            world.set_voxel(
                (world_x + 1) as usize,
                (world_y - 1) as usize,
                VoxelType::Air,
            );
            world.set_voxel(
                (world_x - 1) as usize,
                (world_y + 1) as usize,
                VoxelType::Air,
            );
        }
    }
}
