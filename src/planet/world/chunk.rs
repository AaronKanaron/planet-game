use crate::{
    foliage::variants::tree::Tree,
    planet::{meshing::dual_contouring::ContourCell, world::voxel::VoxelType},
};
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

pub const CHUNK_SIZE: usize = 16;

pub struct Chunk {
    // Simple grid of voxel types
    voxels: Vec<VoxelType>,

    /// Whether this chunk needs to be re-meshed
    dirty: bool,

    /// Whether this chunk has been modified apart from the initial generation
    modified: bool,

    /// Vec<(A, B)> where A is the position of the normal and B is the actual vector
    surface_normals: Vec<(Vec2, Vec2)>,

    foliage: Vec<Entity>,
}

impl Chunk {
    /// Generate a new chunk with voxel types for a circular planet
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

                // Generate terrain using noise for surface variation
                let terrain_height = Self::sample_terrain_height(
                    world_x,
                    world_y,
                    distance_from_center,
                    PLANET_RADIUS,
                    noise,
                );

                // Check if we're inside the planet (accounting for terrain variation)
                let is_inside_planet = distance_from_center < terrain_height;

                let material = if !is_inside_planet {
                    VoxelType::Air
                } else {
                    // Determine material type based on depth and cave systems
                    let depth_from_surface = terrain_height - distance_from_center;

                    // Check for caves
                    if Self::is_cave(world_x, world_y, depth_from_surface, noise) {
                        VoxelType::Air
                    } else {
                        Self::determine_material(
                            depth_from_surface,
                            distance_from_center,
                            PLANET_RADIUS,
                            world_x,
                            world_y,
                            noise,
                        )
                    }
                };

                voxels.push(material);
            }
        }

        Chunk {
            voxels,
            dirty: true,
            modified: false,
            surface_normals: Vec::new(),
            foliage: Vec::new(),
        }
    }

    /// Sample terrain height at a given position (how far the terrain extends from center)
    fn sample_terrain_height(
        world_x: f64,
        world_y: f64,
        distance_from_center: f64,
        planet_radius: f64,
        noise: &Perlin,
    ) -> f64 {
        // Base planet radius
        let base_radius = planet_radius;

        // Sample multi-scale terrain for surface displacement
        let terrain_displacement = Self::sample_terrain_displacement(
            world_x,
            world_y,
            distance_from_center,
            planet_radius,
            noise,
        );

        // Apply displacement to create terrain variation
        base_radius + terrain_displacement
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
        let ridge_noise1 =
            noise.get([world_x / ridge_scale + 500.0, world_y / ridge_scale + 500.0]);
        let ridge_noise2 =
            noise.get([world_x / ridge_scale + 600.0, world_y / ridge_scale + 600.0]);

        // Create ridged noise by taking absolute value and inverting
        let ridge_effect = (1.0 - (ridge_noise1 * ridge_noise2).abs()) * 60.0;

        let total_displacement = continental_noise
            + regional_noise
            + local_noise
            + detail_noise
            + surface_noise
            + ridge_effect;
        total_displacement * falloff
    }

    /// Check if a position should be a cave
    fn is_cave(world_x: f64, world_y: f64, depth_from_surface: f64, noise: &Perlin) -> bool {
        // Only generate caves below the surface with some minimum depth
        if depth_from_surface < 10.0 {
            return false;
        }

        // Multi-scale cave generation for more interesting cave systems
        let cave_scale_1 = 150.0; // Primary tunnel scale
        let cave_scale_2 = 80.0; // Secondary tunnel scale
        let cave_scale_3 = 300.0; // Large chamber scale

        // Primary cave tunnels
        let cave_noise1 = noise.get([
            world_x / cave_scale_1 + 1000.0,
            world_y / cave_scale_1 + 1000.0,
        ]);
        let cave_noise2 = noise.get([
            world_x / cave_scale_1 + 2000.0,
            world_y / cave_scale_1 + 2000.0,
        ]);

        // Secondary smaller tunnels
        let cave_noise3 = noise.get([
            world_x / cave_scale_2 + 3000.0,
            world_y / cave_scale_2 + 3000.0,
        ]);
        let cave_noise4 = noise.get([
            world_x / cave_scale_2 + 4000.0,
            world_y / cave_scale_2 + 4000.0,
        ]);

        // Large chambers
        let chamber_noise = noise.get([
            world_x / cave_scale_3 + 5000.0,
            world_y / cave_scale_3 + 5000.0,
        ]);

        // Check for primary tunnels
        let primary_tunnel_value = (cave_noise1.abs() + cave_noise2.abs()) * 0.5;
        let primary_threshold = 0.2;

        // Check for secondary tunnels
        let secondary_tunnel_value = (cave_noise3.abs() + cave_noise4.abs()) * 0.5;
        let secondary_threshold = 0.15;

        // Check for chambers
        let chamber_value = chamber_noise.abs();
        let chamber_threshold = 0.25;

        // Return true if any cave system is present
        primary_tunnel_value < primary_threshold
            || secondary_tunnel_value < secondary_threshold
            || chamber_value < chamber_threshold
    }

    /// Determine material type based on depth and position
    fn determine_material(
        depth_from_surface: f64,
        distance_from_center: f64,
        planet_radius: f64,
        world_x: f64,
        world_y: f64,
        noise: &Perlin,
    ) -> VoxelType {
        let distance_ratio = distance_from_center / planet_radius;

        // Core region - very close to center
        if distance_ratio < 0.15 {
            return VoxelType::Core;
        }

        // Deep rock layer
        if depth_from_surface > 50.0 && distance_ratio < 0.8 {
            return VoxelType::Rock;
        }

        // Surface materials
        let surface_noise = noise.get([world_x / 100.0 + 500.0, world_y / 100.0 + 500.0]);

        if depth_from_surface < 20.0 && surface_noise > 0.1 {
            VoxelType::Grass
        } else {
            VoxelType::Dirt
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

                // TODO: Normal here?
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

    pub fn surface_normals(&self) -> &[(Vec2, Vec2)] {
        &self.surface_normals
    }

    pub fn foliage(&self) -> &[Entity] {
        &self.foliage
    }
    pub fn clear_foliage(&mut self) {
        self.foliage.clear();
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

    /// Populate surface normals from dual contouring contour cells
    pub fn populate_surface_normals(
        &mut self,
        contour_cells: Vec<ContourCell>,
        chunk_x: i32,
        chunk_y: i32,
    ) {
        use crate::planet::world::voxel::VOXEL_SIZE;

        self.surface_normals.clear();
        self.surface_normals.reserve(contour_cells.len());

        let chunk_world_x = chunk_x as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE;
        let chunk_world_y = chunk_y as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE;

        for cell in contour_cells {
            let cell_world_x = chunk_world_x + cell.x as f32 * VOXEL_SIZE;
            let cell_world_y = chunk_world_y + cell.y as f32 * VOXEL_SIZE;

            let absolute_vertex_pos = Vec2::new(
                cell_world_x + cell.interior_vertex.x * VOXEL_SIZE,
                cell_world_y + cell.interior_vertex.y * VOXEL_SIZE,
            );

            self.surface_normals
                .push((absolute_vertex_pos, cell.normal));
        }
    }

    /// Generate foliage for this chunk using a deterministic seed
    pub fn generate_foliage(
        &mut self,
        commands: &mut Commands,
        chunk_x: i32,
        chunk_y: i32,
        world_seed: u64,
    ) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Create deterministic random number generator for this chunk
        let mut hasher = DefaultHasher::new();
        world_seed.hash(&mut hasher);
        chunk_x.hash(&mut hasher);
        chunk_y.hash(&mut hasher);
        let chunk_seed = hasher.finish();

        // Simple LCG for deterministic randomness
        let mut rng_state = chunk_seed;
        let mut next_random = || {
            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
            rng_state
        };

        // Generate trees based on surface normals
        for &(pos, normal) in &self.surface_normals {
            // Use deterministic chance based on position and chunk seed
            let pos_hash = (pos.x as u64) ^ ((pos.y as u64) << 32);
            let foliage_chance = ((next_random() ^ pos_hash) as f64) / (u64::MAX as f64);

            if foliage_chance < 0.05 {
                // 1% chance for foliage
                let tree = commands
                    .spawn(Tree {
                        transform: Transform::from_translation(Vec3::new(
                            pos.x, pos.y, 0.0, // Assuming flat terrain
                        ))
                        .with_rotation(Quat::from_rotation_z(normal.to_angle())),
                    })
                    .id();

                self.foliage.push(tree);
            }
        }
    }
}
