use crate::{
    planet::{
        meshing::dual_contouring::{DualContouring, SharedVertexRegistry},
        rendering::culling::ChunkCullingBox,
        world::{
            chunk::{CHUNK_SIZE, Chunk},
            voxel::{VOXEL_SIZE, VoxelType},
        },
    },
    utils::debug::plugin::DebugRotate,
};
use bevy::{platform::collections::HashMap, prelude::*};
use noise::Perlin;

#[derive(Resource)]
pub struct World {
    /// Loaded chunks by their world position
    pub loaded_chunks: HashMap<(i32, i32), Chunk>,

    pub noise: Perlin,

    /// World seed for deterministic generation
    pub seed: u64,

    /// Shared vertex registry for border vertices
    pub vertex_registry: SharedVertexRegistry,
}

impl World {
    pub fn new() -> Self {
        Self {
            loaded_chunks: HashMap::new(),
            noise: Perlin::new(42),
            seed: rand::random(),
            vertex_registry: SharedVertexRegistry::new(),
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

        if let Some(chunk) = self.loaded_chunks.get_mut(&(chunk_x, chunk_y)) {
            chunk.set_voxel(local_x as usize, local_y as usize, vtype);

            self.mark_neighboring_chunks_dirty(chunk_x, chunk_y, local_x, local_y);
        } else {
            // Generate the chunk ?
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

    // pub fn mark_all_chunks_dirty(&mut self) {
    //     for chunk in self.loaded_chunks.values_mut() {
    //         chunk.mark_dirty();
    //     }
    // }

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

    /// Get mutable access to the shared vertex registry
    pub fn get_vertex_registry_mut(&mut self) -> &mut SharedVertexRegistry {
        &mut self.vertex_registry
    }

    /// Get immutable access to the shared vertex registry
    pub fn get_vertex_registry(&self) -> &SharedVertexRegistry {
        &self.vertex_registry
    }

    /// Mark neighboring chunks as dirty when a voxel near chunk boundaries is modified.
    /// This ensures that mesh boundaries are updated correctly for dual contouring.
    fn mark_neighboring_chunks_dirty(
        &mut self,
        chunk_x: i32,
        chunk_y: i32,
        local_x: i32,
        local_y: i32,
    ) {
        // Edge cases
        if local_x == 0 {
            self.mark_chunk_dirty(chunk_x - 1, chunk_y);
        }
        if local_x == (CHUNK_SIZE as i32 - 1) {
            self.mark_chunk_dirty(chunk_x + 1, chunk_y);
        }
        if local_y == 0 {
            self.mark_chunk_dirty(chunk_x, chunk_y - 1);
        }
        if local_y == (CHUNK_SIZE as i32 - 1) {
            self.mark_chunk_dirty(chunk_x, chunk_y + 1);
        }

        // Corner casses,
        if local_x == 0 && local_y == 0 {
            self.mark_chunk_dirty(chunk_x - 1, chunk_y - 1);
        }
        if local_x == (CHUNK_SIZE as i32 - 1) && local_y == 0 {
            self.mark_chunk_dirty(chunk_x + 1, chunk_y - 1);
        }
        if local_x == 0 && local_y == (CHUNK_SIZE as i32 - 1) {
            self.mark_chunk_dirty(chunk_x - 1, chunk_y + 1);
        }
        if local_x == (CHUNK_SIZE as i32 - 1) && local_y == (CHUNK_SIZE as i32 - 1) {
            self.mark_chunk_dirty(chunk_x + 1, chunk_y + 1);
        }
    }

    pub fn chunk_size() -> usize {
        CHUNK_SIZE
    }

    /// Load chunks within the given culling box
    pub fn load_chunks_in_box(
        &mut self,
        commands: &mut Commands,
        culling_box: &ChunkCullingBox,
    ) -> Vec<(i32, i32)> {
        let (min_chunk_x, min_chunk_y, max_chunk_x, max_chunk_y) =
            culling_box.get_chunk_bounds(CHUNK_SIZE);

        let mut loaded_chunks = Vec::new();

        for chunk_x in min_chunk_x..=max_chunk_x {
            for chunk_y in min_chunk_y..=max_chunk_y {
                if culling_box.contains_chunk(chunk_x, chunk_y, CHUNK_SIZE)
                    && !self.loaded_chunks.contains_key(&(chunk_x, chunk_y))
                {
                    let mut chunk = Chunk::generate(chunk_x, chunk_y, &self.noise);
                    self.loaded_chunks.insert((chunk_x, chunk_y), chunk);
                    loaded_chunks.push((chunk_x, chunk_y));
                }
            }
        }

        // The insertion of chunk normals have to be done separatly because it's
        // dependent on neighboring chunks
        for chunk in loaded_chunks.iter() {
            let (chunk_x, chunk_y) = chunk.clone();
            let (contour_cells, _border_intersections) =
                DualContouring::find_contour_cells_with_borders(
                    &*self,
                    chunk_x,
                    chunk_y,
                    &mut SharedVertexRegistry::new(),
                );
            let chunk = self.loaded_chunks.get_mut(&(chunk_x, chunk_y)).unwrap();
            chunk.populate_surface_normals(contour_cells, chunk_x, chunk_y);
            chunk.generate_foliage(commands, chunk_x, chunk_y, self.seed);
        }

        loaded_chunks
    }

    /// Unload chunks outside the given culling box
    pub fn unload_chunks_outside_box(
        &mut self,
        commands: &mut Commands,
        culling_box: &ChunkCullingBox,
    ) -> Vec<(i32, i32)> {
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

        // Despawn foliage entities
        for &(chunk_x, chunk_y) in &chunks_to_remove {
            if let Some(chunk) = self.loaded_chunks.get_mut(&(chunk_x, chunk_y)) {
                for entity in chunk.foliage() {
                    commands.entity(*entity).despawn();
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

    /// Finds the closest normal to `position`, useful for snapping items
    /// to planet surface.
    pub fn find_closest_normal(
        &mut self,
        position: Vec2,
        max_distance: f32,
    ) -> Option<(Vec2, Vec2)> {
        let mut closest_distance = f32::INFINITY;
        let mut closest_normal = None;
        let mut closest_position = None;

        // Calculate which chunks to check based on cursor position and max distance
        let chunk_size_world = CHUNK_SIZE as f32 * VOXEL_SIZE;
        let (min_chunk_x, max_chunk_x, min_chunk_y, max_chunk_y) = if max_distance.is_infinite() {
            // If infinite distance, check all loaded chunks
            let mut min_x = i32::MAX;
            let mut max_x = i32::MIN;
            let mut min_y = i32::MAX;
            let mut max_y = i32::MIN;

            for &(chunk_x, chunk_y) in self.loaded_chunks.keys() {
                min_x = min_x.min(chunk_x);
                max_x = max_x.max(chunk_x);
                min_y = min_y.min(chunk_y);
                max_y = max_y.max(chunk_y);
            }

            if min_x == i32::MAX {
                // No loaded chunks
                return None;
            }

            (min_x, max_x, min_y, max_y)
        } else {
            (
                ((position.x - max_distance) / chunk_size_world).floor() as i32,
                ((position.x + max_distance) / chunk_size_world).ceil() as i32,
                ((position.y - max_distance) / chunk_size_world).floor() as i32,
                ((position.y + max_distance) / chunk_size_world).ceil() as i32,
            )
        };

        // Check all nearby chunks for preview points
        for chunk_x in min_chunk_x..=max_chunk_x {
            for chunk_y in min_chunk_y..=max_chunk_y {
                if self.loaded_chunks.contains_key(&(chunk_x, chunk_y)) {
                    let Some(chunk) = self.loaded_chunks.get(&(chunk_x, chunk_y)) else {
                        continue;
                    };

                    for (world_pos, normal) in chunk.surface_normals().to_owned() {
                        let distance = position.distance(world_pos);

                        if distance < closest_distance
                            && (max_distance.is_infinite() || distance <= max_distance)
                        {
                            closest_distance = distance;
                            closest_normal = Some(normal);
                            closest_position = Some(world_pos);
                        }
                    }
                }
            }
        }

        if let (Some(normal), Some(position)) = (closest_normal, closest_position) {
            Some((normal, position))
        } else {
            None
        }
    }
}
