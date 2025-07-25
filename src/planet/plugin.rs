use crate::planet::mesh::{MeshRenderer, chunk::Chunk, world::World};
use crate::planet::culling::ChunkCullingBox;
use crate::planet::culling_system::{chunk_culling_system, draw_culling_box_gizmo, culling_box_input_system};
use crate::planet::debug::{setup_debug_ui, update_debug_info};
use crate::planet::planet_material::PlanetMaterialPlugin;
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
        world.mark_all_chunks_dirty();

        commands.insert_resource(world);
        
        // Initialize the culling box resource
        commands.insert_resource(ChunkCullingBox::new(
            Vec2::new(0.0, 0.0), // Center at origin
            Vec2::new(100.0, 100.0), // Half-extents of 100x100 world units
        ));
    }
}

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PlanetMaterialPlugin)
            .add_systems(Startup, (Self::setup, setup_debug_ui))
            .add_systems(Update, (
                chunk_culling_system,
                MeshRenderer::cleanup_unloaded_chunks,
                MeshRenderer::render_mesh,
                draw_culling_box_gizmo,
                culling_box_input_system,
                update_debug_info,
            ).chain()); // Use chain() to ensure proper ordering
    }
}
