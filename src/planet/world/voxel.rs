use bevy::prelude::*;

#[derive(Component)]
pub struct Voxel;

pub const VOXEL_SIZE: f32 = 6.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
