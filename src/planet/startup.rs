use bevy::{platform::collections::HashMap, prelude::*};
use noise::{NoiseFn, Perlin};
use once_cell::sync::Lazy;

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
        // let voxels = vec![VoxelType::Air; width * height];
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

static PERLIN: Lazy<Perlin> = Lazy::new(|| Perlin::new(42));

pub fn generate_chunk(cx: i32, cy: i32) -> Chunk {
    let mut voxels = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);
    let scale = 50.0;

    for dx in 0..CHUNK_SIZE {
        for dy in 0..CHUNK_SIZE {
            // Calculate world coordinates centered in the voxel
            let world_x = (cx * CHUNK_SIZE as i32 + dx as i32) as f64 + 0.5;
            let world_y = (cy * CHUNK_SIZE as i32 + dy as i32) as f64 + 0.5;

            let noise_value = PERLIN.get([world_x / scale, world_y / scale]);

            let voxel = if noise_value > 0.0 {
                VoxelType::Rock
            } else if noise_value > -0.5 {
                VoxelType::Dirt
            } else {
                VoxelType::Air
            };
            voxels.push(voxel);
        }
    }

    Chunk {
        position: (cx, cy),
        voxels,
    }
}