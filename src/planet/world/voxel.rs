use bevy::prelude::*;

#[derive(Component)]
pub struct Voxel;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VoxelType {
    Air,
    Rock,
    Dirt,
    Grass,
    Core,
}

/// Signed Distance Field voxel data
/// Negative values = inside solid material
/// Positive values = outside (air)
/// Zero = exactly on the surface
#[derive(Clone, Copy, Debug)]
pub struct VoxelSDF {
    /// The signed distance value
    pub distance: f32,
    /// The material type at this location
    pub material: VoxelType,
}

impl VoxelSDF {
    pub fn new(distance: f32, material: VoxelType) -> Self {
        Self { distance, material }
    }

    /// Create an air voxel with positive distance
    pub fn air(distance: f32) -> Self {
        Self {
            distance: distance.max(0.001), // Ensure air is always positive
            material: VoxelType::Air,
        }
    }

    /// Create a solid voxel with negative distance
    pub fn solid(distance: f32, material: VoxelType) -> Self {
        Self {
            distance: -distance.abs(), // Ensure solid is always negative
            material,
        }
    }

    /// Check if this voxel represents solid material
    pub fn is_solid(&self) -> bool {
        self.distance <= 0.0
    }

    /// Check if this voxel represents air
    pub fn is_air(&self) -> bool {
        self.distance > 0.0
    }

    /// Get the material type
    /// Uses the stored material directly since we set it correctly during chunk generation
    pub fn get_material(&self) -> VoxelType {
        self.material
    }
}
