use crate::planet::{
    debug::{
        DebugState, create_chunk_boundaries, debug_input_system, setup_debug_ui,
        toggle_chunk_boundary_visibility, toggle_wireframe_visibility, track_chunk_updates,
        update_debug_info, update_triangle_count,
    },
    meshing::mesh_renderer::{MeshRenderer, VOXEL_SIZE},
    rendering::{
        culling::ChunkCullingBox,
        culling_system::{chunk_culling_system, culling_box_input_system, draw_culling_box_gizmo},
        materials::PlanetMaterialPlugin,
    },
    world::{chunk::CHUNK_SIZE, chunk::Chunk, chunk_world::World},
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

        // Initialize the culling box resource
        commands.insert_resource(ChunkCullingBox::new(
            Vec2::new(0.0, 0.0), // Center at origin
            Vec2::new(
                VOXEL_SIZE * CHUNK_SIZE as f32,
                VOXEL_SIZE * CHUNK_SIZE as f32,
            ), // 2x2 chunks
        ));

        // Initialize debug state resource
        commands.insert_resource(DebugState::default());
    }
}

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PlanetMaterialPlugin)
            .add_systems(Startup, (Self::setup, setup_debug_ui))
            .add_systems(
                Update,
                (
                    chunk_culling_system,
                    track_chunk_updates, // Move this BEFORE mesh rendering
                    MeshRenderer::cleanup_unloaded_chunks,
                    MeshRenderer::render_mesh,
                    update_triangle_count,
                    draw_culling_box_gizmo,
                    culling_box_input_system,
                    debug_input_system,
                    toggle_wireframe_visibility,
                    toggle_chunk_boundary_visibility,
                    create_chunk_boundaries,
                    update_debug_info,
                )
                    .chain(),
            ); // Use chain() to ensure proper ordering
    }
}
