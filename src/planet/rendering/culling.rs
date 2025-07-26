use crate::planet::meshing::mesh_renderer::VOXEL_SIZE;
use bevy::prelude::*;

/// Represents a 2D bounding box for chunk culling
#[derive(Resource)]
pub struct ChunkCullingBox {
    /// Center of the bounding box in world coordinates
    pub center: Vec2,
    /// Half-extents of the bounding box in world coordinates
    pub half_extents: Vec2,
    /// Additional padding around the culling box for preloading chunks
    pub padding: f32,
    /// Whether the culling box is enabled
    pub enabled: bool,
}

impl Default for ChunkCullingBox {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            half_extents: Vec2::new(200.0, 200.0), // Default 400x400 world units
            padding: 50.0,                         // Default padding
            enabled: true,
        }
    }
}

impl ChunkCullingBox {
    /// Create a new culling box with the given center and half-extents
    pub fn new(center: Vec2, half_extents: Vec2) -> Self {
        Self {
            center,
            half_extents,
            padding: 50.0,
            enabled: true,
        }
    }
    /// Check if a chunk at the given chunk coordinates is within the bounding box
    pub fn contains_chunk(&self, chunk_x: i32, chunk_y: i32, chunk_size: usize) -> bool {
        if !self.enabled {
            return true; // If disabled, all chunks are considered "inside"
        }

        // Convert chunk coordinates to world coordinates
        let chunk_world_size = (chunk_size as f32) * VOXEL_SIZE;

        // Calculate chunk bounds - chunks start at their coordinate * size
        let chunk_min_x = chunk_x as f32 * chunk_world_size;
        let chunk_max_x = chunk_min_x + chunk_world_size;
        let chunk_min_y = chunk_y as f32 * chunk_world_size;
        let chunk_max_y = chunk_min_y + chunk_world_size;

        // Calculate the bounding box bounds with padding
        let min_x = self.center.x - self.half_extents.x - self.padding;
        let max_x = self.center.x + self.half_extents.x + self.padding;
        let min_y = self.center.y - self.half_extents.y - self.padding;
        let max_y = self.center.y + self.half_extents.y + self.padding;

        // Check for overlap using AABB intersection
        !(chunk_max_x <= min_x
            || chunk_min_x >= max_x
            || chunk_max_y <= min_y
            || chunk_min_y >= max_y)
    }

    /// Get the minimum and maximum chunk coordinates that could be within the bounding box
    pub fn get_chunk_bounds(&self, chunk_size: usize) -> (i32, i32, i32, i32) {
        let chunk_world_size = (chunk_size as f32) * VOXEL_SIZE;

        let min_x = self.center.x - self.half_extents.x - self.padding;
        let max_x = self.center.x + self.half_extents.x + self.padding;
        let min_y = self.center.y - self.half_extents.y - self.padding;
        let max_y = self.center.y + self.half_extents.y + self.padding;

        // Calculate which chunks could potentially intersect with the bounding box
        // We use floor for min and floor for max because chunks are aligned to grid
        let min_chunk_x = (min_x / chunk_world_size).floor() as i32;
        let max_chunk_x = (max_x / chunk_world_size).floor() as i32;
        let min_chunk_y = (min_y / chunk_world_size).floor() as i32;
        let max_chunk_y = (max_y / chunk_world_size).floor() as i32;

        (min_chunk_x, min_chunk_y, max_chunk_x, max_chunk_y)
    }
}
