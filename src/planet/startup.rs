use bevy::{platform::collections::HashMap, prelude::*};
use noise::{NoiseFn, Perlin};

pub const CHUNK_SIZE: usize = 16; // Size of each chunk in voxels (n x n)

pub struct Chunk {
    position: (i32, i32),   // World-space position (chunk coordinates)
    voxels: Vec<VoxelType>, // Size: CHUNK_SIZE * CHUNK_SIZE
}

impl Chunk {
    pub fn set_voxel(&mut self, x: usize, y: usize, vtype: VoxelType) {
        let idx = y * CHUNK_SIZE + x;
        if idx < self.voxels.len() {
            self.voxels[idx] = vtype;
        }
    }

    pub fn get_voxel(&self, x: usize, y: usize) -> VoxelType {
        let idx = y * CHUNK_SIZE + x;
        if idx < self.voxels.len() {
            self.voxels[idx]
        } else {
            VoxelType::Air
        }
    }
}

#[derive(Resource)]
pub struct VoxelWorld {
    pub(crate) width: usize,
    pub(crate) height: usize,
    // pub(crate) voxels: Vec<VoxelType>,
    pub loaded_chunks: HashMap<(i32, i32), Chunk>, // Loaded chunks by their world position
}

impl VoxelWorld {
    pub fn new(width: usize, height: usize) -> Self {
        let voxels = vec![VoxelType::Air; width * height];
        Self {
            width,
            height,
            // voxels,
            loaded_chunks: HashMap::new(),
        }
    }

    pub fn get_voxel(&self, x: i32, y: i32) -> VoxelType {
        let chunk_x = x.div_euclid(CHUNK_SIZE as i32);
        let chunk_y = y.div_euclid(CHUNK_SIZE as i32);
        let local_x = x.rem_euclid(CHUNK_SIZE as i32);
        let local_y = y.rem_euclid(CHUNK_SIZE as i32);

        // Look up the chunk
        if let Some(chunk) = self.loaded_chunks.get(&(chunk_x, chunk_y)) {
            chunk.get_voxel(local_x as usize, local_y as usize)
        } else {
            VoxelType::Air // Or trigger chunk generation
        }
    }

    pub fn set_voxel(&mut self, x: i32, y: i32, vtype: VoxelType) {
        let chunk_x = x.div_euclid(CHUNK_SIZE as i32);
        let chunk_y = y.div_euclid(CHUNK_SIZE as i32);
        let local_x = x.rem_euclid(CHUNK_SIZE as i32);
        let local_y = y.rem_euclid(CHUNK_SIZE as i32);

        // Get the chunk, or optionally generate it
        if let Some(chunk) = self.loaded_chunks.get_mut(&(chunk_x, chunk_y)) {
            chunk.set_voxel(local_x as usize, local_y as usize, vtype)
        } else {
            // Optionally generate and insert chunk
        }
    }

    pub fn get_loaded_chunks(&self) -> impl Iterator<Item = (&(i32, i32), &Chunk)> {
        self.loaded_chunks.iter()
    }

    pub fn load_chunk(&mut self, chunk_x: i32, chunk_y: i32) {
        if !self.loaded_chunks.contains_key(&(chunk_x, chunk_y)) {
            let chunk = generate_chunk(chunk_x, chunk_y);
            self.loaded_chunks.insert((chunk_x, chunk_y), chunk);
        }
    }

    pub fn unload_chunk(&mut self, chunk_x: i32, chunk_y: i32) {
        self.loaded_chunks.remove(&(chunk_x, chunk_y));
    }

    pub fn is_chunk_loaded(&self, chunk_x: i32, chunk_y: i32) -> bool {
        self.loaded_chunks.contains_key(&(chunk_x, chunk_y))
    }

    pub fn chunk_size() -> usize {
        CHUNK_SIZE
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum VoxelType {
    Air,
    Rock,
    Dirt,
    // Sand,
    // Water,
    // Ice,
    // Lava,
    // DeepRock,
    // Ore,
    // Crystal,
    // Obsidian,
    // Grass,
}

pub fn generate_chunk(cx: i32, cy: i32) -> Chunk {
    let mut voxels = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);
    let perlin_noise = Perlin::new(42); // Example noise generator, adjust as needed

    for dx in 0..CHUNK_SIZE {
        for dy in 0..CHUNK_SIZE {
            let world_x = cx * CHUNK_SIZE as i32 + dx as i32;
            let world_y = cy * CHUNK_SIZE as i32 + dy as i32;

            // Example terrain generation logic using Perlin noise
            let noise_value = perlin_noise.get([world_x as f64 / 10.0, world_y as f64 / 10.0]);
            let voxel = if noise_value > 0.0 {
                VoxelType::Rock
            } else {
                VoxelType::Air
            };
            voxels.push(voxel);
        }
    }

    Chunk {
        position: (cx, cy),
        voxels: voxels,
    }
}

// pub fn generate_terrain(world: &mut VoxelWorld) {
//     let noise = Perlin::new(42);
//     let detail_noise = Perlin::new(123);
//     let cave_noise = Perlin::new(456);

//     let center_x = world.width as f32 / 2.0;
//     let center_y = world.height as f32 / 2.0;
//     let base_radius = 80.0;

//     for x in 0..world.width {
//         for y in 0..world.height {
//             let dx = x as f32 - center_x;
//             let dy = y as f32 - center_y;
//             let distance = (dx * dx + dy * dy).sqrt();

//             // Create irregular planet shape using noise
//             let shape_noise = noise.get([x as f64 / 15.0, y as f64 / 15.0]) * 8.0;
//             let effective_radius = base_radius + shape_noise as f32;

//             if distance < effective_radius {
//                 // Height-based terrain layers
//                 let height_factor = 1.0 - (distance / effective_radius);

//                 // Add surface detail
//                 let surface_noise = detail_noise.get([x as f64 / 5.0, y as f64 / 5.0]);
//                 let cave_value = cave_noise.get([x as f64 / 8.0, y as f64 / 8.0]);

//                 // Create caves/air pockets
//                 if cave_value > 0.4 && height_factor < 0.8 {
//                     continue; // Leave as air
//                 }

//                 // Core region (deep rock)
//                 if height_factor > 0.7 {
//                     world.set_voxel(x, y, VoxelType::Rock);
//                 }
//                 // Middle layer with mixed materials
//                 else if height_factor > 0.4 {
//                     if surface_noise > 0.2 {
//                         world.set_voxel(x, y, VoxelType::Rock);
//                     } else {
//                         world.set_voxel(x, y, VoxelType::Dirt);
//                     }
//                 }
//                 // Surface layer - mostly dirt with some rock outcrops
//                 else {
//                     if surface_noise > 0.5 {
//                         world.set_voxel(x, y, VoxelType::Rock);
//                     } else if surface_noise > -0.3 {
//                         world.set_voxel(x, y, VoxelType::Dirt);
//                     }
//                     // Else remains air for surface variation
//                 }
//             }
//         }
//     }
// }
