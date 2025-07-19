use noise::{NoiseFn, Perlin};
use crate::{VoxelType, VoxelWorld};

pub fn generate_terrain(world: &mut VoxelWorld) {
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
