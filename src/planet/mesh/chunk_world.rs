use bevy::{platform::collections::HashMap, prelude::*};
use noise::Perlin;
use crate::planet::mesh::{chunk::{Chunk, CHUNK_SIZE}, VoxelType};

#[derive(Resource)]
pub struct ChunkWorld {
    /// Loaded chunks by their world position
    pub loaded_chunks: HashMap<(i32, i32), Chunk>,

    pub noise: Perlin
}

impl ChunkWorld {
    pub fn new() -> Self {
        Self {
            loaded_chunks: HashMap::new(),
            noise: Perlin::new(42)
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
