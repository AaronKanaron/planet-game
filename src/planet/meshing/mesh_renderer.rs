use bevy::prelude::*;

use crate::planet::{
    meshing::render_voxels::VoxelRenderer,
    rendering::materials::{CoreMaterial, DirtMaterial, GrassMaterial, RockMaterial},
    world::{chunk_world::World, voxel::VoxelType},
};

#[derive(Component)]
pub struct ChunkMesh {
    pub chunk_x: i32,
    pub chunk_y: i32,
}

pub struct MeshRenderer;

impl MeshRenderer {
    /// This updates the planet mesh by only regenerating the dirty, "changed" chunks.
    pub fn render_mesh(
        mut commands: Commands,
        mut world: ResMut<World>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut rock_materials: ResMut<Assets<RockMaterial>>,
        mut dirt_materials: ResMut<Assets<DirtMaterial>>,
        mut grass_materials: ResMut<Assets<GrassMaterial>>,
        mut core_materials: ResMut<Assets<CoreMaterial>>,
        existing_chunks: Query<(Entity, &ChunkMesh)>,
    ) {
        let dirty_chunks = world.get_dirty_chunks();
        if dirty_chunks.is_empty() {
            return;
        }

        // Despawn dirty chunks
        for (entity, chunk_mesh) in existing_chunks.iter() {
            if dirty_chunks.contains(&(chunk_mesh.chunk_x, chunk_mesh.chunk_y)) {
                commands.entity(entity).despawn();
            }
        }

        // Regenerate meshes for dirty chunks only
        for &(chunk_x, chunk_y) in &dirty_chunks {
            // Skip if chunk is no longer loaded (might have been unloaded)
            if !world.is_chunk_loaded(chunk_x, chunk_y) {
                continue;
            }

            let chunk = world.loaded_chunks.get(&(chunk_x, chunk_y)).unwrap();

            // Create separate meshes for each material type
            for &material_type in &[
                VoxelType::Rock,
                VoxelType::Dirt,
                VoxelType::Grass,
                VoxelType::Core,
            ] {
                // Delegate to the voxel renderer
                VoxelRenderer::render_voxels(
                    chunk,
                    chunk_x,
                    chunk_y,
                    material_type,
                    &mut commands,
                    &mut meshes,
                    &mut rock_materials,
                    &mut dirt_materials,
                    &mut grass_materials,
                    &mut core_materials,
                );
            }
            // Remove dirty flag
            world.mark_chunk_clean(chunk_x, chunk_y);
        }
    }
    /// Despawn mesh entities for chunks that are no longer loaded
    pub fn cleanup_unloaded_chunks(
        commands: Commands,
        world: Res<World>,
        existing_chunks: Query<(Entity, &ChunkMesh)>,
    ) {
        VoxelRenderer::cleanup_unloaded_chunks(commands, world, existing_chunks);
    }
}
