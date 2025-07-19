use bevy::prelude::*;

use crate::{planet::{mesh::render_mesh, startup::generate_terrain}, VoxelWorld};

pub struct PlanetPlugin;

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup)
            .add_systems(Update, render_mesh);

    }
}

fn setup(mut commands: Commands) {
    let mut world = VoxelWorld::new(100, 100);
    generate_terrain(&mut world);
    commands.insert_resource(world);
}