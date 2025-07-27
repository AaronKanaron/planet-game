use crate::planet::world::voxel::{VoxelType, VoxelSDF};
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

pub const CHUNK_SIZE: usize = 16;

pub struct Chunk {
    // Changed from VoxelType to VoxelSDF
    voxels: Vec<VoxelSDF>,

    /// Whether this chunk needs to be re-meshed
    dirty: bool,

    /// Whether this chunk has been modified apart from the initial generation
    modified: bool,
}

impl Chunk {
    /// Generate a new chunk with SDF values for a circular planet
    pub fn generate(cx: i32, cy: i32, noise: &Perlin) -> Chunk {
        let mut voxels = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

        // Planet parameters
        const PLANET_CENTER_X: f64 = 0.0;
        const PLANET_CENTER_Y: f64 = 0.0;
        const PLANET_RADIUS: f64 = 300.0;

        for dy in 0..CHUNK_SIZE {
            for dx in 0..CHUNK_SIZE {
                // Calculate exact world coordinates
                let world_x = (cx * CHUNK_SIZE as i32 + dx as i32) as f64;
                let world_y = (cy * CHUNK_SIZE as i32 + dy as i32) as f64;

                // Calculate distance from planet center
                let distance_from_center = ((world_x - PLANET_CENTER_X).powi(2) + 
                                          (world_y - PLANET_CENTER_Y).powi(2)).sqrt();

                // Sample terrain surface distance using noise
                let surface_distance = Self::sample_planet_surface_distance(
                    world_x, world_y, distance_from_center, PLANET_RADIUS, noise
                );

                // Sample cave system to modify the SDF
                let cave_distance = Self::sample_cave_sdf(
                    world_x, world_y, distance_from_center, PLANET_RADIUS, surface_distance, noise
                );

                // Combine surface and cave distances (union operation)
                let final_distance = surface_distance.max(cave_distance);

                // Determine material type based on depth and location
                let material = Self::determine_material_from_sdf(
                    final_distance, distance_from_center, PLANET_RADIUS, world_x, world_y, noise
                );

                // Create SDF voxel
                let voxel_sdf = VoxelSDF::new(final_distance, material);
                voxels.push(voxel_sdf);
            }
        }

        // TODO: Update surface grass logic for SDF
        // Self::add_surface_grass(&mut voxels, cx, cy);

        Chunk {
            voxels,
            dirty: true,
            modified: false,
        }
    }

    /// Sample planet surface as a signed distance field
    /// Returns negative values inside the planet, positive outside
    fn sample_planet_surface_distance(world_x: f64, world_y: f64, distance_from_center: f64, 
                                     planet_radius: f64, noise: &Perlin) -> f32 {
        // Base distance to sphere
        let base_distance = distance_from_center - planet_radius;
        
        // Sample multi-scale terrain for surface displacement
        let terrain_displacement = Self::sample_terrain_displacement(world_x, world_y, distance_from_center, planet_radius, noise);
        
        // Apply displacement to create terrain variation
        (base_distance - terrain_displacement) as f32
    }

    /// Sample terrain displacement for surface variation
    fn sample_terrain_displacement(world_x: f64, world_y: f64, distance_from_center: f64, 
                                 planet_radius: f64, noise: &Perlin) -> f64 {
        let distance_ratio = distance_from_center / planet_radius;
        
        // Reduce displacement far from planet
        let falloff = if distance_ratio < 1.5 {
            (1.5 - distance_ratio).max(0.0)
        } else {
            0.0
        };
        
        // Continental scale noise
        let continental_scale = 8000.0;
        let continental_noise = noise.get([world_x / continental_scale, world_y / continental_scale]) * 50.0;
        
        // Regional scale noise (mountains, valleys)
        let regional_scale = 2000.0;
        let regional_noise = noise.get([world_x / regional_scale + 100.0, world_y / regional_scale + 100.0]) * 30.0;
        
        // Local detail noise
        let local_scale = 500.0;
        let local_noise = noise.get([world_x / local_scale + 200.0, world_y / local_scale + 200.0]) * 15.0;
        
        // Fine detail noise
        let detail_scale = 100.0;
        let detail_noise = noise.get([world_x / detail_scale + 300.0, world_y / detail_scale + 300.0]) * 5.0;
        
        // Surface roughness
        let surface_scale = 50.0;
        let surface_noise = noise.get([world_x / surface_scale + 400.0, world_y / surface_scale + 400.0]) * 2.0;
        
        let total_displacement = continental_noise + regional_noise + local_noise + detail_noise + surface_noise;
        total_displacement * falloff
    }

    /// Sample cave system as SDF (returns positive values inside caves)
    fn sample_cave_sdf(world_x: f64, world_y: f64, distance_from_center: f64, 
                       planet_radius: f64, surface_distance: f32, noise: &Perlin) -> f32 {
        // Only generate caves inside the planet
        if surface_distance > 0.0 {
            return f32::NEG_INFINITY; // No caves in air
        }
        
        let distance_ratio = distance_from_center / planet_radius;
        
        // Don't generate caves in core or too close to surface
        if distance_ratio < 0.3 || distance_ratio > 0.9 {
            return f32::NEG_INFINITY;
        }
        
        // Cave tunnel noise
        let cave_scale = 200.0;
        let cave_noise1 = noise.get([world_x / cave_scale + 1000.0, world_y / cave_scale + 1000.0]);
        let cave_noise2 = noise.get([world_x / cave_scale + 2000.0, world_y / cave_scale + 2000.0]);
        
        // Create cave tunnels using noise
        let cave_value = (cave_noise1.abs() + cave_noise2.abs()) * 0.5;
        let cave_threshold = 0.15; // Adjust for cave density
        
        if cave_value < cave_threshold {
            let cave_radius = 8.0; // Cave tunnel radius
            let distance_to_cave_center = (cave_threshold - cave_value) / cave_threshold * cave_radius;
            distance_to_cave_center as f32
        } else {
            f32::NEG_INFINITY
        }
    }

    /// Determine material type from SDF value and position
    fn determine_material_from_sdf(distance: f32, distance_from_center: f64, planet_radius: f64,
                                  world_x: f64, world_y: f64, noise: &Perlin) -> VoxelType {
        if distance > 0.0 {
            return VoxelType::Air;
        }
        
        let distance_ratio = distance_from_center / planet_radius;
        let depth = -distance; // How deep inside the planet
        
        // Core region
        if distance_ratio < 0.15 {
            return VoxelType::Core;
        }
        
        // Rock layer (deep)
        if depth > 20.0 || distance_ratio < 0.4 {
            return VoxelType::Rock;
        }
        
        // Surface materials
        let surface_noise = noise.get([world_x / 100.0 + 500.0, world_y / 100.0 + 500.0]);
        
        if depth < 3.0 && surface_noise > 0.1 {
            VoxelType::Grass
        } else {
            VoxelType::Dirt
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
        
        // Don't generate caves in the core region
        const CORE_RADIUS_RATIO: f64 = 0.2;
        if distance_ratio < CORE_RADIUS_RATIO * 1.1 { // Give some buffer around core
            return false;
        }
        
        // Don't generate caves too close to the planet's edge
        if distance_ratio > 0.95 {
            return false;
        }

        // Cave entrance noise - larger scale for main cave locations
        let cave_entrance_scale = 200.0;
        let cave_entrance_noise = noise.get([world_x / cave_entrance_scale + 1000.0, 
                                           world_y / cave_entrance_scale + 1000.0]);
        
        // Surface cave entrances - rare but noticeable
        let surface_cave_threshold = 0.2; // Higher = fewer caves
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

    /// Add grass to surface areas of the chunk
    fn add_surface_grass(voxels: &mut Vec<VoxelType>, chunk_x: i32, chunk_y: i32) {
        const PLANET_CENTER_X: f64 = 0.0;
        const PLANET_CENTER_Y: f64 = 0.0;
        
        // Simple approach: check each voxel and see if it has air neighbors in the outward direction
        for dy in 0..CHUNK_SIZE {
            for dx in 0..CHUNK_SIZE {
                let idx = dy * CHUNK_SIZE + dx;
                
                if idx >= voxels.len() || voxels[idx] != VoxelType::Dirt {
                    continue; // Only convert dirt to grass (not core, rock, or air)
                }
                
                // Calculate world coordinates for this voxel
                let world_x = (chunk_x * CHUNK_SIZE as i32 + dx as i32) as f64;
                let world_y = (chunk_y * CHUNK_SIZE as i32 + dy as i32) as f64;
                
                // Calculate direction from planet center to this voxel
                let dx_from_center = world_x - PLANET_CENTER_X;
                let dy_from_center = world_y - PLANET_CENTER_Y;
                let distance_from_center = (dx_from_center * dx_from_center + dy_from_center * dy_from_center).sqrt();
                
                if distance_from_center < 1.0 {
                    continue; // Skip if too close to center
                }
                
                // Check if this dirt voxel has any air neighbors within the same chunk
                let mut has_air_neighbor = false;
                
                // Check all 8 directions around this voxel
                for neighbor_dy in -1..=1i32 {
                    for neighbor_dx in -1..=1i32 {
                        if neighbor_dx == 0 && neighbor_dy == 0 {
                            continue; // Skip self
                        }
                        
                        let neighbor_x = dx as i32 + neighbor_dx;
                        let neighbor_y = dy as i32 + neighbor_dy;
                        
                        // Only check neighbors within the current chunk
                        if neighbor_x >= 0 && neighbor_x < CHUNK_SIZE as i32 && 
                           neighbor_y >= 0 && neighbor_y < CHUNK_SIZE as i32 {
                            let neighbor_idx = (neighbor_y as usize) * CHUNK_SIZE + (neighbor_x as usize);
                            if neighbor_idx < voxels.len() && voxels[neighbor_idx] == VoxelType::Air {
                                has_air_neighbor = true;
                                break;
                            }
                        }
                    }
                    if has_air_neighbor {
                        break;
                    }
                }
                
                // If this dirt voxel has air neighbors, it's potentially on the surface
                if has_air_neighbor {
                    // Additional check: make sure we're not too deep underground
                    // Count how many solid voxels are between this position and the direction away from planet center
                    let dir_x = dx_from_center / distance_from_center;
                    let dir_y = dy_from_center / distance_from_center;
                    
                    let mut solid_count = 0;
                    let mut found_air = false;
                    
                    // Sample outward from this voxel within the chunk
                    for step in 1..=6 {
                        let check_x = world_x + dir_x * step as f64;
                        let check_y = world_y + dir_y * step as f64;
                        
                        // Convert to local chunk coordinates
                        let local_x = ((check_x as i32) - (chunk_x * CHUNK_SIZE as i32)) as i32;
                        let local_y = ((check_y as i32) - (chunk_y * CHUNK_SIZE as i32)) as i32;
                        
                        // If still within chunk bounds, check the voxel
                        if local_x >= 0 && local_x < CHUNK_SIZE as i32 && 
                           local_y >= 0 && local_y < CHUNK_SIZE as i32 {
                            let check_idx = (local_y as usize) * CHUNK_SIZE + (local_x as usize);
                            if check_idx < voxels.len() {
                                if voxels[check_idx] == VoxelType::Air {
                                    found_air = true;
                                    break;
                                } else {
                                    solid_count += 1;
                                }
                            }
                        } else {
                            // Outside chunk bounds - stop checking
                            break;
                        }
                    }
                    
                    // Only place grass if we have air neighbors and we're close to the surface
                    // (not buried under many solid voxels)
                    if found_air || solid_count <= 2 {
                        voxels[idx] = VoxelType::Grass;
                    }
                }
            }
        }
    }

    /// Determine voxel type based on terrain height and planet characteristics
    fn determine_voxel_type(terrain_height: f64, distance_from_center: f64, planet_radius: f64,
                          world_x: f64, world_y: f64, noise: &Perlin) -> VoxelType {
        // If we're clearly outside the planet, it's space (air)
        if terrain_height < 0.02 {
            return VoxelType::Air;
        }
        
        // Calculate distance ratio for core detection
        let distance_ratio = distance_from_center / planet_radius;
        
        // Core region - almost perfect circle in the center
        const CORE_RADIUS_RATIO: f64 = 0.2; // Increased to 20% for visibility
        if distance_ratio <= CORE_RADIUS_RATIO {
            return VoxelType::Core;
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
            // Create new SDF voxel, preserving distance but changing material
            let new_distance = if vtype == VoxelType::Air {
                old_voxel.distance.abs() // Make positive for air
            } else {
                -old_voxel.distance.abs() // Make negative for solid
            };
            self.voxels[idx] = VoxelSDF::new(new_distance, vtype);

            let is_changed = old_voxel.material != vtype;
            if is_changed {
                self.dirty = true;
                self.modified = true;
            }
        }
    }

    pub fn get_voxel(&self, x: usize, y: usize) -> VoxelType {
        let idx = y * CHUNK_SIZE + x;

        if idx < self.voxels.len() {
            self.voxels[idx].get_material()
        } else {
            VoxelType::Air
        }
    }

    /// Get SDF value at the given position
    pub fn get_voxel_sdf(&self, x: usize, y: usize) -> VoxelSDF {
        let idx = y * CHUNK_SIZE + x;

        if idx < self.voxels.len() {
            self.voxels[idx]
        } else {
            VoxelSDF::air(1.0) // Default to air with positive distance
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
