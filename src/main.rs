use bevy::prelude::*;
use bevy::window::WindowPlugin;
use bevy::input::ButtonInput;
use noise::{NoiseFn, Perlin};

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
    voxels: Vec<Vec<VoxelType>>,
}

impl VoxelWorld {
    fn new(width: usize, height: usize) -> Self {
        let voxels = vec![vec![VoxelType::Air; width]; height];
        Self { width, height, voxels }
    }

    fn get_voxel(&self, x: usize, y: usize) -> VoxelType {
        if x < self.width && y < self.height {
            self.voxels[y][x]
        } else {
            VoxelType::Air
        }
    }

    fn set_voxel(&mut self, x: usize, y: usize, voxel_type: VoxelType) {
        if x < self.width && y < self.height {
            self.voxels[y][x] = voxel_type;
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevyism".to_string(),
                resolution: (800.0, 600.0).into(),
                ..default()
            }),
            ..Default::default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (render_voxels, handle_input))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    let mut world = VoxelWorld::new(100, 100);
    generate_terrain(&mut world);

    commands.insert_resource(world);
}

fn generate_terrain(world: &mut VoxelWorld) {
    let noise = Perlin::new(42);
    let center_x = world.width as f32 / 2.0;
    let center_y = world.height as f32 / 2.0;
    let radius = 30.0;

    for x in 0..world.width {
        for y in 0..world.height {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < radius {
                let noise_value = noise.get([x as f64 / 10.0, y as f64 / 10.0]);

                if distance < radius - 5.0 {
                    world.set_voxel(x, y, VoxelType::Rock);
                } else if noise_value > 0.0 {
                    world.set_voxel(x, y, VoxelType::Dirt);
                }
            }
        }
    }
}

fn render_voxels(
    mut commands: Commands,
    world: Res<VoxelWorld>,
    existing_voxels: Query<Entity, With<Voxel>>,
) {
    for entity in existing_voxels.iter() {
        commands.entity(entity).despawn();
    }

    for x in 0..world.width {
        for y in 0..world.height {
            let voxel = world.get_voxel(x, y);
            if voxel != VoxelType::Air {
                let color = match voxel {
                    VoxelType::Rock => Color::srgb(0.3, 0.3, 0.3),
                    VoxelType::Dirt => Color::srgb(0.6, 0.4, 0.2),
                    VoxelType::Air => continue,
                };

                let world_x = x as f32 * 8.0 - 400.0;
                let world_y = 300.0 - y as f32 * 8.0;

                commands.spawn((
                    Sprite {
                        color,
                        custom_size: Some(Vec2::new(8.0, 8.0)),
                        ..default()
                    },
                    Transform::from_xyz(world_x, world_y, 0.0),
                    Voxel,
                ));
            }
        }
    }
}

#[derive(Component)]
struct Voxel;

fn update() {
    // Add your update logic here
    // This function will be called every frame
}

fn handle_input(
    mut world: ResMut<VoxelWorld>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    if !mouse_input.just_pressed(MouseButton::Left) {
        return;
    }

    let window = match windows.single() {
        Ok(w) => w,
        Err(_) => return,
    };

    let Some(cursor_pos) = window.cursor_position() else { return; };

    let Ok((camera, camera_transform)) = camera_query.single() else { return; };

    if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
        let world_x = ((world_pos.x + 400.0) / 8.0) as isize;
        let world_y = ((300.0 - world_pos.y) / 8.0) as isize;

        if world_x >= 0 && world_y >= 0 {
            world.set_voxel(world_x as usize, world_y as usize, VoxelType::Air);
        }
    }
}