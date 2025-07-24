use bevy::{platform::collections::HashMap, prelude::*};
use noise::{NoiseFn, Perlin};
use once_cell::sync::Lazy;

pub const CHUNK_SIZE: usize = 16; 

pub struct Chunk {
    voxels: Vec<VoxelType>,
    /// Whether this chunk needs to be re-meshed
    pub dirty: bool,
}

impl Chunk {
    pub fn set_voxel(&mut self, x: usize, y: usize, vtype: VoxelType) {
        let idx = y * CHUNK_SIZE + x;

        if idx < self.voxels.len() {
            let old_voxel = self.voxels[idx];
            self.voxels[idx] = vtype;

            let is_changed = old_voxel != vtype;
            if is_changed { self.dirty = true; }
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

    /// Returns whether this chunk needs to be re-meshed
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

#[derive(Resource)]
pub struct VoxelWorld {
    pub loaded_chunks: HashMap<(i32, i32), Chunk>, // Loaded chunks by their world position
}

impl VoxelWorld {
    pub fn new() -> Self {
        Self {
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
            chunk.set_voxel(local_x as usize, local_y as usize, vtype);

            // Mark adjacent chunks as dirty if we're near the border
            // This ensures that mesh boundaries are updated correctly
            if local_x == 0 && chunk_x > 0 {
                if let Some(adj_chunk) = self.loaded_chunks.get_mut(&(chunk_x - 1, chunk_y)) {
                    adj_chunk.mark_dirty();
                }
            }
            if local_x == (CHUNK_SIZE as i32 - 1) {
                if let Some(adj_chunk) = self.loaded_chunks.get_mut(&(chunk_x + 1, chunk_y)) {
                    adj_chunk.mark_dirty();
                }
            }
            if local_y == 0 && chunk_y > 0 {
                if let Some(adj_chunk) = self.loaded_chunks.get_mut(&(chunk_x, chunk_y - 1)) {
                    adj_chunk.mark_dirty();
                }
            }
            if local_y == (CHUNK_SIZE as i32 - 1) {
                if let Some(adj_chunk) = self.loaded_chunks.get_mut(&(chunk_x, chunk_y + 1)) {
                    adj_chunk.mark_dirty();
                }
            }
        } else {
            // Optionally generate and insert chunk
        }
    }

    pub fn is_chunk_loaded(&self, chunk_x: i32, chunk_y: i32) -> bool {
        self.loaded_chunks.contains_key(&(chunk_x, chunk_y))
    }

    pub fn mark_chunk_clean(&mut self, chunk_x: i32, chunk_y: i32) {
        if let Some(chunk) = self.loaded_chunks.get_mut(&(chunk_x, chunk_y)) {
            chunk.mark_clean();
        }
    }

    pub fn get_dirty_chunks(&self) -> Vec<(i32, i32)> {
        self.loaded_chunks
            .iter()
            .filter(|(_, chunk)| chunk.is_dirty())
            .map(|(&pos, _)| pos)
            .collect()
    }

    pub fn mark_all_chunks_dirty(&mut self) {
        for chunk in self.loaded_chunks.values_mut() {
            chunk.mark_dirty();
        }
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

fn sample_terrain_height(world_x: f64, world_y: f64) -> f64 {
    let base_scale = 50.0;
    
    // Single noise sample - should be in range [-1, 1]
    let height = PERLIN.get([world_x / base_scale, world_y / base_scale]);
    
    height
}

pub fn generate_chunk(cx: i32, cy: i32) -> Chunk {
    let mut voxels = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

    for dy in 0..CHUNK_SIZE {
        for dx in 0..CHUNK_SIZE {
            // Calculate exact world coordinates (no offset needed for seamless chunks)
            let world_x = (cx * CHUNK_SIZE as i32 + dx as i32) as f64;
            let world_y = (cy * CHUNK_SIZE as i32 + dy as i32) as f64;

            // Sample the base terrain height
            let terrain_height = sample_terrain_height(world_x, world_y);
            
            // Create smoother transitions with multiple noise layers
            let detail_noise = PERLIN.get([world_x / 20.0, world_y / 20.0]) * 0.3;
            let combined_height = terrain_height + detail_noise;
            
            // Smoother thresholds that should create gradual transitions
            let voxel = if combined_height < -0.3 {
                VoxelType::Air
            } else if combined_height < 0.3 {
                // Add some randomness in the middle range
                if combined_height < 0.0 {
                    VoxelType::Dirt
                } else {
                    // Mix dirt and rock in transition zone
                    let dirt_noise = PERLIN.get([world_x / 15.0 + 1000.0, world_y / 15.0 + 1000.0]);
                    if dirt_noise > 0.2 {
                        VoxelType::Rock
                    } else {
                        VoxelType::Dirt
                    }
                }
            } else {
                VoxelType::Rock
            };
            
            voxels.push(voxel);
        }
    }

    Chunk {
        voxels,
        dirty: true,
    }
}
