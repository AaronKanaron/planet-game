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

    /// Get the number of loaded chunks
    pub fn loaded_chunk_count(&self) -> usize {
        self.loaded_chunks.len()
    }

    /// Convert a world position to chunk coordinates
    pub fn world_pos_to_chunk_coords(position: Vec2) -> (i32, i32) {
        let chunk_size_world = CHUNK_SIZE as f32 * VOXEL_SIZE;
        let chunk_x = (position.x / chunk_size_world).floor() as i32;
        let chunk_y = (position.y / chunk_size_world).floor() as i32;
        (chunk_x, chunk_y)
    }

    /// Get the closest surface normal to a given position.
    /// Returns Vec2::ZERO if no surface normal is found.
    /// Only checks chunks that are close to the input position.
    pub fn get_closest_surface_normal(&self, position: Vec2) -> (Vec2, Vec2) {
        let mut closest_distance = f32::INFINITY;
        let mut closest_normal = Vec2::ZERO;
        let mut closest_normal_position = Vec2::ZERO;

        let (center_chunk_x, center_chunk_y) = Self::world_pos_to_chunk_coords(position);

        // Check the chunk containing the position and its 8 neighbors
        for dx in -1..=1 {
            for dy in -1..=1 {
                let chunk_x = center_chunk_x + dx;
                let chunk_y = center_chunk_y + dy;

                if let Some(chunk) = self.loaded_chunks.get(&(chunk_x, chunk_y)) {
                    for &(world_pos, normal) in chunk.surface_normals() {
                        let distance = position.distance(world_pos);
                        if distance < closest_distance {
                            closest_distance = distance;
                            closest_normal = normal;
                            closest_normal_position = world_pos;
                        }
                    }
                }
            }
        }

        (closest_normal_position, closest_normal)
    }
}
