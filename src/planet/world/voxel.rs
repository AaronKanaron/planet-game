use bevy::prelude::*;

#[derive(Component)]
pub struct Voxel;

#[derive(Clone, Copy, PartialEq)]
pub enum VoxelType {
    Air,
    Rock,
    Dirt,
}
