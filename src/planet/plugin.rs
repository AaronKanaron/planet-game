use bevy::prelude::*;

use crate::{planet::{mesh::MeshRenderer, startup::generate_chunk}, VoxelWorld};

pub struct PlanetPlugin;

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup)
            .add_systems(Update, MeshRenderer::render_mesh);

    }
}

fn setup(mut commands: Commands) {
    let mut world = VoxelWorld::new(200, 200);  // Changed from 100x100 to 200x200
    // PlanetGenerator::new(100.0, 42).generate_planet(&mut world);
    // generate_terrain(&mut world);
    for cx in 0..8 {
        for cy in 0..8 {
            let chunk = generate_chunk(cx as i32, cy as i32);
            world.loaded_chunks.insert((cx as i32, cy as i32), chunk);
        }
    }
    // generate_chunk(0, 0);
    commands.insert_resource(world);
}