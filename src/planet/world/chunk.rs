use crate::planet::world::voxel::{VoxelSDF, VoxelType};
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
    /// Smooth maximum function for SDF composition - creates smooth blends instead of sharp edges
    fn smooth_max(a: f32, b: f32, k: f32) -> f32 {
        // Smooth maximum operation for SDF blending
        // When k=0, this becomes regular max(a,b)
        // Higher k values create smoother transitions
        let h = (k - (a - b).abs()).max(0.0) / k;
        a.max(b) + h * h * k * 0.25
    }

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
                let distance_from_center = ((world_x - PLANET_CENTER_X).powi(2)
                    + (world_y - PLANET_CENTER_Y).powi(2))
                .sqrt();

                // Sample terrain surface distance using noise
                let surface_distance = Self::sample_planet_surface_distance(
                    world_x,
                    world_y,
                    distance_from_center,
                    PLANET_RADIUS,
                    noise,
                );

                // Sample cave system to modify the SDF
                let cave_distance_opt = Self::sample_cave_sdf(
                    world_x,
                    world_y,
                    distance_from_center,
                    PLANET_RADIUS,
                    surface_distance,
                    noise,
                );

                // Combine surface and cave distances using proper SDF composition
                // Only carve caves where they actually exist
                let final_distance = if let Some(cave_distance) = cave_distance_opt {
                    // Proper SDF composition: caves carve out material by creating air where caves exist
                    // For caves (negative cave_distance = inside cave), we want air (positive distance)
                    // Use SDF "subtraction" operation: surface - cave
                    let combined = if cave_distance < 0.0 {
                        // Inside cave: blend smoothly from surface to air
                        // Convert cave_distance (negative) to air_distance (positive)
                        let air_distance = -cave_distance; // Now positive inside caves
                        
                        // Use smooth maximum for SDF subtraction/carving
                        // This creates smooth transitions instead of sharp cuts
                        let smoothing_factor = 2.0; // Controls transition smoothness
                        Self::smooth_max(surface_distance, air_distance, smoothing_factor)
                    } else {
                        // Outside cave: use original surface distance
                        surface_distance
                    };
                    
                    // Debug cave generation for a few chunks
                    if dx == 0 && dy == 0 && (cx.abs() <= 2 && cy.abs() <= 2) && cave_distance < 0.0 {
                        let air_distance = -cave_distance;
                    }
                    
                    combined
                } else {
                    // No caves here, just use surface distance
                    surface_distance
                };

                // Determine material type based on final distance and location
                let material = if final_distance > 0.0 {
                    // Positive distance = Air
                    VoxelType::Air
                } else {
                    // Negative distance = solid material
                    // Determine which solid material based on depth and location
                    let solid_material = Self::determine_material_from_sdf(
                        final_distance,
                        distance_from_center,
                        PLANET_RADIUS,
                        world_x,
                        world_y,
                        noise,
                    );
                                        
                    solid_material
                };

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
    fn sample_planet_surface_distance(
        world_x: f64,
        world_y: f64,
        distance_from_center: f64,
        planet_radius: f64,
        noise: &Perlin,
    ) -> f32 {
        // Base distance to sphere
        let base_distance = distance_from_center - planet_radius;

        // Sample multi-scale terrain for surface displacement
        let terrain_displacement = Self::sample_terrain_displacement(
            world_x,
            world_y,
            distance_from_center,
            planet_radius,
            noise,
        );

        // Apply displacement to create terrain variation
        (base_distance - terrain_displacement) as f32
    }

    /// Sample terrain displacement for surface variation
    fn sample_terrain_displacement(
        world_x: f64,
        world_y: f64,
        distance_from_center: f64,
        planet_radius: f64,
        noise: &Perlin,
    ) -> f64 {
        let distance_ratio = distance_from_center / planet_radius;

        // Allow more dramatic terrain further from planet center
        // Gentle falloff to allow mountains to extend well beyond base radius
        let falloff = if distance_ratio < 2.0 {
            (2.0 - distance_ratio).max(0.0).powf(0.5) // Gentler falloff curve
        } else {
            0.0
        };

        // Continental scale noise - major landmasses and ocean basins
        let continental_scale = 8000.0;
        let continental_noise =
            noise.get([world_x / continental_scale, world_y / continental_scale]) * 120.0;

        // Regional scale noise (mountains, valleys) - much more dramatic
        let regional_scale = 2000.0;
        let regional_noise = noise.get([
            world_x / regional_scale + 100.0,
            world_y / regional_scale + 100.0,
        ]) * 80.0;

        // Local detail noise - significant hills and valleys
        let local_scale = 500.0;
        let local_noise =
            noise.get([world_x / local_scale + 200.0, world_y / local_scale + 200.0]) * 40.0;

        // Fine detail noise - pronounced ridges and gullies
        let detail_scale = 100.0;
        let detail_noise = noise.get([
            world_x / detail_scale + 300.0,
            world_y / detail_scale + 300.0,
        ]) * 15.0;

        // Surface roughness - terrain texture
        let surface_scale = 50.0;
        let surface_noise = noise.get([
            world_x / surface_scale + 400.0,
            world_y / surface_scale + 400.0,
        ]) * 5.0;

        // Mountain ridge noise - creates sharp peaks and deep valleys
        let ridge_scale = 1200.0;
        let ridge_noise1 = noise.get([world_x / ridge_scale + 500.0, world_y / ridge_scale + 500.0]);
        let ridge_noise2 = noise.get([world_x / ridge_scale + 600.0, world_y / ridge_scale + 600.0]);
        
        // Create ridged noise by taking absolute value and inverting
        let ridge_effect = (1.0 - (ridge_noise1 * ridge_noise2).abs()) * 60.0;

        let total_displacement = continental_noise + regional_noise + local_noise + detail_noise + surface_noise + ridge_effect;
        total_displacement * falloff
    }

    /// Sample cave system as SDF 
    /// Returns the distance to the nearest cave surface
    /// Negative values = inside cave (air), positive = inside solid rock
    fn sample_cave_sdf(
        world_x: f64,
        world_y: f64,
        distance_from_center: f64,
        planet_radius: f64,
        surface_distance: f32,
        noise: &Perlin,
    ) -> Option<f32> {
        // Only generate caves inside the planet (where surface_distance is negative)
        if surface_distance > 0.0 {
            return None; // No caves in air
        }

        let distance_ratio = distance_from_center / planet_radius;

        // Don't generate caves in core or too close to surface
        // Adjusted to allow caves closer to surface for easier visibility
        if distance_ratio < 0.1 || distance_ratio > 0.95 {
            return None; // No caves here
        }

        // Multi-scale cave generation for more interesting cave systems
        let cave_scale_1 = 150.0; // Primary tunnel scale
        let cave_scale_2 = 80.0;  // Secondary tunnel scale
        let cave_scale_3 = 300.0; // Large chamber scale

        // Primary cave tunnels
        let cave_noise1 = noise.get([world_x / cave_scale_1 + 1000.0, world_y / cave_scale_1 + 1000.0]);
        let cave_noise2 = noise.get([world_x / cave_scale_1 + 2000.0, world_y / cave_scale_1 + 2000.0]);
        
        // Secondary smaller tunnels
        let cave_noise3 = noise.get([world_x / cave_scale_2 + 3000.0, world_y / cave_scale_2 + 3000.0]);
        let cave_noise4 = noise.get([world_x / cave_scale_2 + 4000.0, world_y / cave_scale_2 + 4000.0]);

        // Large chambers
        let chamber_noise = noise.get([world_x / cave_scale_3 + 5000.0, world_y / cave_scale_3 + 5000.0]);

        // Calculate distance to primary tunnels
        let primary_tunnel_value = (cave_noise1.abs() + cave_noise2.abs()) * 0.5;
        let primary_threshold = 0.2; // Increased threshold for more caves
        let primary_radius = 8.0_f32; // Larger radius

        // Calculate distance to secondary tunnels
        let secondary_tunnel_value = (cave_noise3.abs() + cave_noise4.abs()) * 0.5;
        let secondary_threshold = 0.15; // Increased threshold for more caves
        let secondary_radius = 4.0_f32; // Larger radius

        // Calculate distance to chambers
        let chamber_value = chamber_noise.abs();
        let chamber_threshold = 0.25; // Increased threshold for more caves
        let chamber_radius = 20.0_f32; // Larger radius

        // Calculate SDF for each cave type
        let primary_distance = if primary_tunnel_value < primary_threshold {
            let t = ((primary_threshold - primary_tunnel_value) / primary_threshold) as f32;
            -(primary_radius * t) // Negative inside tunnel
        } else {
            primary_radius * (((primary_tunnel_value - primary_threshold) / (1.0 - primary_threshold)) as f32)
        };

        let secondary_distance = if secondary_tunnel_value < secondary_threshold {
            let t = ((secondary_threshold - secondary_tunnel_value) / secondary_threshold) as f32;
            -(secondary_radius * t) // Negative inside tunnel
        } else {
            secondary_radius * (((secondary_tunnel_value - secondary_threshold) / (1.0 - secondary_threshold)) as f32)
        };

        let chamber_distance = if chamber_value < chamber_threshold {
            let t = ((chamber_threshold - chamber_value) / chamber_threshold) as f32;
            -(chamber_radius * t) // Negative inside chamber
        } else {
            chamber_radius * (((chamber_value - chamber_threshold) / (1.0 - chamber_threshold)) as f32)
        };

        // Combine cave systems using SDF union (min operation for negative distances)
        let combined_cave_distance = primary_distance.min(secondary_distance).min(chamber_distance);

        // Only return cave distance if it's actually creating a cave (negative)
        if combined_cave_distance < 0.0 {
            Some(combined_cave_distance)
        } else {
            None
        }
    }

    /// Determine material type from SDF value and position (for solid materials only)
    fn determine_material_from_sdf(
        distance: f32,
        distance_from_center: f64,
        planet_radius: f64,
        world_x: f64,
        world_y: f64,
        noise: &Perlin,
    ) -> VoxelType {
        // This function should only be called for negative distances (solid materials)
        debug_assert!(distance <= 0.0, "determine_material_from_sdf called with positive distance");

        let distance_ratio = distance_from_center / planet_radius;
        let depth = -distance; // How deep inside the planet

        // Core region - very close to center
        if distance_ratio < 0.15 {
            return VoxelType::Core;
        }

        // Rock layer (deep) - much more generous thresholds to allow surface materials
        // Only classify as deep rock if we're really far from surface
        if depth > 300.0 && distance_ratio < 0.8 {
            return VoxelType::Rock;
        }

        // Surface materials - much more generous threshold
        let surface_noise = noise.get([world_x / 100.0 + 500.0, world_y / 100.0 + 500.0]);

        if depth < 200.0 && surface_noise > 0.1 {
            VoxelType::Grass
        } else {
            VoxelType::Dirt
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
