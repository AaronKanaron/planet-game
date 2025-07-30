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

impl VoxelType {
    /// Check if this voxel type represents solid material
    pub fn is_solid(&self) -> bool {
        match self {
            VoxelType::Air => false,
            _ => true,
        }
    }
}
