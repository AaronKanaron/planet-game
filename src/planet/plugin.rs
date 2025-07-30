use crate::planet::{
    debug::{DebugPlugin, DebugState},
    meshing::mesh_renderer::MeshRenderer,
    rendering::{
        culling::ChunkCullingBox,
        culling_system::{chunk_culling_system, culling_box_input_system, draw_culling_box_gizmo},
        materials::PlanetMaterialPlugin,
    },
    world::{chunk::Chunk, chunk_world::World},
};

use bevy::prelude::*;

pub struct PlanetPlugin;

impl PlanetPlugin {
    fn setup(mut commands: Commands) {
        let mut world = World::new();

        // Load initial chunks in a smaller area since we'll use culling
        for cx in 0..4 {
            for cy in 0..4 {
                let chunk = Chunk::generate(cx as i32, cy as i32, &world.noise);
                world.loaded_chunks.insert((cx as i32, cy as i32), chunk);
            }
        }

        // Mark all chunks as dirty so they get rendered on the first frame
        // world.mark_all_chunks_dirty();

        commands.insert_resource(world);

        // Initialize the culling box resource - start close to planet surface
        // Position the culling box closer to the planet surface to see terrain details
        let culling_box = ChunkCullingBox::new(Vec2::new(280.0, 0.0), Vec2::new(100.0, 100.0));
        commands.insert_resource(culling_box);

        // Initialize debug state resource
        commands.insert_resource(DebugState::default());
    }
}

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((PlanetMaterialPlugin, DebugPlugin))
            .add_systems(Startup, Self::setup)
            .add_systems(
                Update,
                (
                    chunk_culling_system,
                    MeshRenderer::cleanup_unloaded_chunks,
                    MeshRenderer::render_mesh,
                    draw_culling_box_gizmo,
                    culling_box_input_system,
                )
                    .chain(),
            ); // Use chain() to ensure proper ordering
    }
}
