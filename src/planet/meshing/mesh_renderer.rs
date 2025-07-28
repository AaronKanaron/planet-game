use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

use crate::planet::{
    meshing::dual_contourer::DualContourer,
    rendering::materials::{CoreMaterial, DirtMaterial, GrassMaterial, RockMaterial},
    world::{
        chunk_world::World,
        voxel::{Voxel, VoxelType},
    },
};

/// Voxel size in world units
pub const VOXEL_SIZE: f32 = 6.0;

#[derive(Component)]
pub struct ChunkMesh {
    pub chunk_x: i32,
    pub chunk_y: i32,
}

pub struct MeshRenderer;

impl MeshRenderer {
    /// Despawn mesh entities for chunks that are no longer loaded
    pub fn cleanup_unloaded_chunks(
        mut commands: Commands,
        world: Res<World>,
        existing_chunks: Query<(Entity, &ChunkMesh)>,
    ) {
        for (entity, chunk_mesh) in existing_chunks.iter() {
            if !world.is_chunk_loaded(chunk_mesh.chunk_x, chunk_mesh.chunk_y) {
                commands.entity(entity).despawn();
            }
        }
    }

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

            for &material_type in &[
                VoxelType::Rock,
                VoxelType::Dirt,
                VoxelType::Grass,
                VoxelType::Core,
            ] {
                let (vertices, indices) =
                    DualContourer::generate_chunk_mesh(&world, chunk_x, chunk_y, material_type);
                
                // Debug mesh generation
                if !vertices.is_empty() {
                    println!("Generated mesh for {:?} at chunk ({}, {}) with {} vertices, {} indices", 
                            material_type, chunk_x, chunk_y, vertices.len(), indices.len());
                }
                
                if !vertices.is_empty() && !indices.is_empty() {
                    let mut filled_mesh = Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::RENDER_WORLD,
                    );

                    // Generate normals (all facing forward for 2D)
                    let normals = Self::generate_mesh_normals(&vertices);
                    filled_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());
                    filled_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone());
                    filled_mesh.insert_indices(Indices::U32(indices.clone()));

                    // Calculate mesh size for shader normalization
                    let mesh_size = Self::calculate_mesh_size(&vertices);

                    // Spawn filled mesh with different material based on type
                    match material_type {
                        VoxelType::Rock => {
                            // Use custom rock material with WGSL shader - brighter base color
                            let rock_material = RockMaterial {
                                color: LinearRgba::rgb(0.5, 0.5, 0.5), // Brighter gray for more visible shader effects
                                mesh_size,
                            };

                            commands.spawn((
                                Mesh2d(meshes.add(filled_mesh)),
                                MeshMaterial2d(rock_materials.add(rock_material)),
                                Transform::default(),
                                Voxel,
                                ChunkMesh { chunk_x, chunk_y },
                            ));
                        }
                        VoxelType::Dirt => {
                            let dirt_material = DirtMaterial {
                                color: LinearRgba::rgb(0.4, 0.26, 0.19),
                                mesh_size,
                            };
                            commands.spawn((
                                Mesh2d(meshes.add(filled_mesh)),
                                MeshMaterial2d(dirt_materials.add(dirt_material)),
                                Transform::default(),
                                Voxel,
                                ChunkMesh { chunk_x, chunk_y },
                            ));
                        }
                        VoxelType::Grass => {
                            let grass_material = GrassMaterial {
                                color: LinearRgba::rgb(0.0, 0.6, 0.0), // Green color for grass
                                mesh_size,
                            };
                            commands.spawn((
                                Mesh2d(meshes.add(filled_mesh)),
                                MeshMaterial2d(grass_materials.add(grass_material)),
                                Transform::default(),
                                Voxel,
                                ChunkMesh { chunk_x, chunk_y },
                            ));
                        }
                        VoxelType::Core => {
                            let core_material = CoreMaterial {
                                color: LinearRgba::rgb(0.1, 0.1, 0.1), // Very dark gray/black for core
                                mesh_size,
                            };
                            commands.spawn((
                                Mesh2d(meshes.add(filled_mesh)),
                                MeshMaterial2d(core_materials.add(core_material)),
                                Transform::default(),
                                Voxel,
                                ChunkMesh { chunk_x, chunk_y },
                            ));
                        }
                        VoxelType::Air => {
                            continue;
                        }
                    }
                }
            }

            // Remove dirty flag
            world.mark_chunk_clean(chunk_x, chunk_y);
        }
    }

    fn generate_mesh_normals(vertices: &Vec<[f32; 3]>) -> Vec<[f32; 3]> {
        vec![[0.0, 0.0, 1.0]; vertices.len()]
    }

    /// Calculate the AABB size of the mesh vertices
    fn calculate_mesh_size(vertices: &Vec<[f32; 3]>) -> Vec3 {
        if vertices.is_empty() {
            return Vec3::ONE;
        }

        let mut min_bounds = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max_bounds = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        for vertex in vertices {
            let pos = Vec3::new(vertex[0], vertex[1], vertex[2]);
            min_bounds = min_bounds.min(pos);
            max_bounds = max_bounds.max(pos);
        }

        let size = max_bounds - min_bounds;
        // Ensure minimum size to avoid division by zero
        Vec3::new(size.x.max(1.0), size.y.max(1.0), size.z.max(1.0))
    }
}
