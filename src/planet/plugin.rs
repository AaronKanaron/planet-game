use crate::planet::mesh::{MeshRenderer, chunk::Chunk, chunk_world::ChunkWorld};
use bevy::prelude::*;

pub struct PlanetPlugin;

impl PlanetPlugin {
    fn setup(mut commands: Commands) {
        let mut world = ChunkWorld::new();

        for cx in 0..8 {
            for cy in 0..8 {
                let chunk = Chunk::generate(cx as i32, cy as i32, &world.noise);
                world.loaded_chunks.insert((cx as i32, cy as i32), chunk);
            }
        }

        // Mark all chunks as dirty so they get rendered on the first frame
        world.mark_all_chunks_dirty();

        commands.insert_resource(world);
    }
}

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::setup)
            .add_systems(Update, MeshRenderer::render_mesh);
    }
}
