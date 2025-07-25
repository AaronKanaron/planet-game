use crate::planet::rendering::culling::ChunkCullingBox;
use crate::planet::world::{
    chunk::{CHUNK_SIZE, Chunk},
    voxel::VoxelType,
};
use bevy::{platform::collections::HashMap, prelude::*};
use noise::Perlin;

#[derive(Resource)]
pub struct World {
    /// Loaded chunks by their world position
    pub loaded_chunks: HashMap<(i32, i32), Chunk>,

    pub noise: Perlin,
}

impl World {
    pub fn new() -> Self {
        Self {
            loaded_chunks: HashMap::new(),
            noise: Perlin::new(42),
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
            // This means that the vvoxel that is being set is in a chunk that does not exist yet.
            // Optionally generate and insert chunk
        }
    }

    pub fn is_chunk_loaded(&self, chunk_x: i32, chunk_y: i32) -> bool {
        self.loaded_chunks.contains_key(&(chunk_x, chunk_y))
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

    pub fn mark_chunk_dirty(&mut self, chunk_x: i32, chunk_y: i32) {
        if let Some(chunk) = self.loaded_chunks.get_mut(&(chunk_x, chunk_y)) {
            chunk.mark_dirty();
        }
    }

    pub fn mark_chunk_clean(&mut self, chunk_x: i32, chunk_y: i32) {
        if let Some(chunk) = self.loaded_chunks.get_mut(&(chunk_x, chunk_y)) {
            chunk.mark_clean();
        }
    }

    pub fn chunk_size() -> usize {
        CHUNK_SIZE
    }

    /// Load chunks within the given culling box
    pub fn load_chunks_in_box(&mut self, culling_box: &ChunkCullingBox) -> Vec<(i32, i32)> {
        let (min_chunk_x, min_chunk_y, max_chunk_x, max_chunk_y) =
            culling_box.get_chunk_bounds(CHUNK_SIZE);

        let mut loaded_chunks = Vec::new();

        for chunk_x in min_chunk_x..=max_chunk_x {
            for chunk_y in min_chunk_y..=max_chunk_y {
                if culling_box.contains_chunk(chunk_x, chunk_y, CHUNK_SIZE)
                    && !self.loaded_chunks.contains_key(&(chunk_x, chunk_y))
                {
                    let chunk = Chunk::generate(chunk_x, chunk_y, &self.noise);
                    self.loaded_chunks.insert((chunk_x, chunk_y), chunk);
                    loaded_chunks.push((chunk_x, chunk_y));
                }
            }
        }

        loaded_chunks
    }

    /// Unload chunks outside the given culling box
    pub fn unload_chunks_outside_box(&mut self, culling_box: &ChunkCullingBox) -> Vec<(i32, i32)> {
        let chunks_to_remove: Vec<(i32, i32)> = self
            .loaded_chunks
            .keys()
            .filter(|&&(chunk_x, chunk_y)| {
                !culling_box.contains_chunk(chunk_x, chunk_y, CHUNK_SIZE)
            })
            .copied()
            .collect();

        // Mark neighboring chunks as dirty before removing chunks
        for &(chunk_x, chunk_y) in &chunks_to_remove {
            // Mark all adjacent chunks as dirty if they exist
            let neighbors = [
                (chunk_x - 1, chunk_y), // left
                (chunk_x + 1, chunk_y), // right
                (chunk_x, chunk_y - 1), // bottom
                (chunk_x, chunk_y + 1), // top
            ];

            for &(nx, ny) in &neighbors {
                if self.loaded_chunks.contains_key(&(nx, ny)) {
                    self.mark_chunk_dirty(nx, ny);
                }
            }
        }

        for &chunk_pos in &chunks_to_remove {
            self.loaded_chunks.remove(&chunk_pos);
        }

        chunks_to_remove
    }

    /// Get all currently loaded chunk positions
    // pub fn get_loaded_chunk_positions(&self) -> Vec<(i32, i32)> {
    //     self.loaded_chunks.keys().copied().collect()
    // }

    /// Get the number of loaded chunks
    pub fn loaded_chunk_count(&self) -> usize {
        self.loaded_chunks.len()
    }
}
