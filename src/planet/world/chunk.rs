use crate::planet::world::voxel::VoxelType;
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

pub const CHUNK_SIZE: usize = 16;

pub struct Chunk {
    voxels: Vec<VoxelType>,

    /// Whether this chunk needs to be re-meshed
    dirty: bool,

    /// Whether this chunk has been modified apart from the initial generation
    modified: bool,
}

impl Chunk {
    /// Generate a new chunk with multi-scale noise for a circular planet
    pub fn generate(cx: i32, cy: i32, noise: &Perlin) -> Chunk {
        let mut voxels = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

        // Planet parameters
        const PLANET_CENTER_X: f64 = 0.0;
        const PLANET_CENTER_Y: f64 = 0.0;
        const PLANET_RADIUS: f64 = 300.0; // More reasonable planet size

        for dy in 0..CHUNK_SIZE {
            for dx in 0..CHUNK_SIZE {
                // Calculate exact world coordinates
                let world_x = (cx * CHUNK_SIZE as i32 + dx as i32) as f64;
                let world_y = (cy * CHUNK_SIZE as i32 + dy as i32) as f64;

                // Calculate distance from planet center
                let distance_from_center = ((world_x - PLANET_CENTER_X).powi(2) + 
                                          (world_y - PLANET_CENTER_Y).powi(2)).sqrt();

                // Sample multi-scale terrain height
                let terrain_height = Self::sample_planet_terrain(world_x, world_y, distance_from_center, 
                                                               PLANET_RADIUS, noise);

                // Check for caves
                let is_cave = Self::sample_cave_system(world_x, world_y, distance_from_center, 
                                                     PLANET_RADIUS, terrain_height, noise);

                // Determine voxel type based on terrain height, distance, and caves
                let voxel = if is_cave {
                    VoxelType::Air
                } else {
                    Self::determine_voxel_type(terrain_height, distance_from_center, 
                                             PLANET_RADIUS, world_x, world_y, noise)
                };

                voxels.push(voxel);
            }
        }

        Chunk {
            voxels,
            dirty: true,
            modified: false,
        }
    }

    /// Sample multi-scale terrain for planetary generation
    fn sample_planet_terrain(world_x: f64, world_y: f64, distance_from_center: f64, 
                           planet_radius: f64, noise: &Perlin) -> f64 {
        // Base circular falloff - creates the planet shape
        let distance_ratio = distance_from_center / planet_radius;
        let base_falloff = (1.0 - distance_ratio.min(1.0)).max(0.0);
        
        // Apply smooth falloff curve
        let circular_base = base_falloff.powf(2.0); // Smooth edge falloff
        
        // Continental scale noise (very large features) - increased amplitude
        let continental_scale = 8000.0;
        let continental_noise = noise.get([world_x / continental_scale, world_y / continental_scale]) * 0.6;
        
        // Regional scale noise (mountains, valleys) - much stronger
        let regional_scale = 2000.0;
        let regional_noise = noise.get([world_x / regional_scale + 100.0, world_y / regional_scale + 100.0]) * 0.4;
        
        // Local detail noise (hills, small features) - increased
        let local_scale = 500.0;
        let local_noise = noise.get([world_x / local_scale + 200.0, world_y / local_scale + 200.0]) * 0.2;
        
        // Fine detail noise - more pronounced
        let detail_scale = 100.0;
        let detail_noise = noise.get([world_x / detail_scale + 300.0, world_y / detail_scale + 300.0]) * 0.1;
        
        // Surface roughness for irregular coastlines
        let surface_scale = 50.0;
        let surface_noise = noise.get([world_x / surface_scale + 400.0, world_y / surface_scale + 400.0]) * 0.05;
        
        // Combine noise layers
        let terrain_variation = continental_noise + regional_noise + local_noise + detail_noise + surface_noise;
        
        // Apply terrain variation in two ways:
        // 1. Modulate the base sphere (preserves planetary shape but adds variation)
        let sphere_variation = circular_base * (1.0 + terrain_variation * 0.8); // Increased variation
        
        // 2. Add significant elevation that can extend well beyond the base sphere
        let elevation_boost = if distance_ratio < 1.2 { // Allow terrain beyond the base radius
            terrain_variation * 0.6 * (1.0 - distance_ratio.powf(0.3)) // Strong elevation, gradual falloff
        } else {
            0.0
        };
        
        let final_height = sphere_variation + elevation_boost;
        
        final_height
    }

    /// Sample cave system for underground caverns and surface entrances
    fn sample_cave_system(world_x: f64, world_y: f64, distance_from_center: f64, 
                         planet_radius: f64, terrain_height: f64, noise: &Perlin) -> bool {
        // Only generate caves within the planet (not in space)
        if terrain_height < 0.02 {
            return false;
        }

        // Distance ratio for cave probability
        let distance_ratio = distance_from_center / planet_radius;
        
        // Don't generate caves too close to the planet's edge
        if distance_ratio > 0.95 {
            return false;
        }

        // Cave entrance noise - larger scale for main cave locations
        let cave_entrance_scale = 200.0;
        let cave_entrance_noise = noise.get([world_x / cave_entrance_scale + 1000.0, 
                                           world_y / cave_entrance_scale + 1000.0]);
        
        // Surface cave entrances - rare but noticeable
        let surface_cave_threshold = 0.7; // Higher = fewer caves
        let is_surface_cave_area = cave_entrance_noise > surface_cave_threshold;
        
        if is_surface_cave_area {
            // Fine detail for cave entrance shape
            let entrance_detail_scale = 30.0;
            let entrance_detail = noise.get([world_x / entrance_detail_scale + 1500.0, 
                                           world_y / entrance_detail_scale + 1500.0]);
            
            // Create cave entrances that are larger near the threshold
            let cave_size_factor = (cave_entrance_noise - surface_cave_threshold) / (1.0 - surface_cave_threshold);
            let entrance_threshold = 0.3 - (cave_size_factor * 0.4); // Larger caves have lower threshold
            
            if entrance_detail > entrance_threshold {
                return true;
            }
        }

        // Underground cave systems - more common deeper underground
        if terrain_height > 0.1 { // Only in areas with some solid material
            // Depth factor - more caves deeper underground
            let depth_factor = (terrain_height - 0.1).min(0.6) / 0.6; // 0.0 to 1.0
            
            // Underground cave noise - different scale and offset
            let underground_scale = 80.0;
            let underground_noise = noise.get([world_x / underground_scale + 2000.0, 
                                             world_y / underground_scale + 2000.0]);
            
            // Cave network noise - creates connected cave systems
            let network_scale = 150.0;
            let network_noise = noise.get([world_x / network_scale + 2500.0, 
                                         world_y / network_scale + 2500.0]);
            
            // Combine noises for more interesting cave shapes
            let combined_cave_noise = (underground_noise + network_noise * 0.5) / 1.5;
            
            // Cave threshold that increases with depth
            let underground_threshold = 0.4 - (depth_factor * 0.3); // More caves deeper
            
            if combined_cave_noise > underground_threshold {
                // Add some fine detail to cave edges
                let cave_detail_scale = 25.0;
                let cave_detail = noise.get([world_x / cave_detail_scale + 3000.0, 
                                           world_y / cave_detail_scale + 3000.0]);
                
                return cave_detail > -0.2; // Most areas in cave zones are caves
            }
        }

        false
    }

    /// Determine voxel type based on terrain height and planet characteristics
    fn determine_voxel_type(terrain_height: f64, _distance_from_center: f64, _planet_radius: f64,
                          world_x: f64, world_y: f64, noise: &Perlin) -> VoxelType {
        // If we're clearly outside the planet, it's space (air)
        if terrain_height < 0.02 {
            return VoxelType::Air;
        }
        
        // Surface areas should be mostly dirt with some rock variation
        if terrain_height < 0.5 {
            // Surface layer - mostly dirt with some rock patches
            let material_noise = noise.get([world_x / 150.0 + 500.0, world_y / 150.0 + 500.0]);
            if material_noise > 0.4 { // Higher threshold = more dirt
                VoxelType::Rock
            } else {
                VoxelType::Dirt
            }
        } else if terrain_height < 0.8 {
            // Higher elevation - mix of dirt and rock, but still plenty of dirt
            let material_noise = noise.get([world_x / 100.0 + 600.0, world_y / 100.0 + 600.0]);
            if material_noise > 0.1 { // Balanced mix
                VoxelType::Rock
            } else {
                VoxelType::Dirt
            }
        } else {
            // Very high peaks - mostly rock but still some dirt
            let material_noise = noise.get([world_x / 80.0 + 700.0, world_y / 80.0 + 700.0]);
            if material_noise > -0.2 {
                VoxelType::Rock
            } else {
                VoxelType::Dirt
            }
        }
    }

    pub fn set_voxel(&mut self, x: usize, y: usize, vtype: VoxelType) {
        let idx = y * CHUNK_SIZE + x;

        if idx < self.voxels.len() {
            let old_voxel = self.voxels[idx];
            self.voxels[idx] = vtype;

            let is_changed = old_voxel != vtype;
            if is_changed {
                self.dirty = true;
                self.modified = true;
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
