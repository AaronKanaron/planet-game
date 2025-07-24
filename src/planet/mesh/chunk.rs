use crate::planet::mesh::VoxelType;
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

pub const CHUNK_SIZE: usize = 16;

pub struct Chunk {
    voxels: Vec<VoxelType>,

    /// Whether this chunk needs to be re-meshed
    dirty: bool,
}

impl Chunk {
    /// Generate a new chunk with Perlin noise
    pub fn generate(cx: i32, cy: i32, noise: &Perlin) -> Chunk {
        let mut voxels = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

        for dy in 0..CHUNK_SIZE {
            for dx in 0..CHUNK_SIZE {
                // Calculate exact world coordinates (no offset needed for seamless chunks)
                let world_x = (cx * CHUNK_SIZE as i32 + dx as i32) as f64;
                let world_y = (cy * CHUNK_SIZE as i32 + dy as i32) as f64;

                // Sample the base terrain height
                let terrain_height = Self::sample_terrain_height(world_x, world_y, noise);

                // Create smoother transitions with multiple noise layers
                let detail_noise = noise.get([world_x / 20.0, world_y / 20.0]) * 0.3;
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
                        let dirt_noise =
                            noise.get([world_x / 15.0 + 1000.0, world_y / 15.0 + 1000.0]);
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

    /// Sample the terrain height using Perlin noise
    fn sample_terrain_height(world_x: f64, world_y: f64, noise: &Perlin) -> f64 {
        let base_scale = 50.0;

        // Single noise sample - should be in range [-1, 1]
        let height = noise.get([world_x / base_scale, world_y / base_scale]);

        height
    }

    pub fn set_voxel(&mut self, x: usize, y: usize, vtype: VoxelType) {
        let idx = y * CHUNK_SIZE + x;

        if idx < self.voxels.len() {
            let old_voxel = self.voxels[idx];
            self.voxels[idx] = vtype;

            let is_changed = old_voxel != vtype;
            if is_changed {
                self.dirty = true;
            }
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
