use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

use crate::planet::{
    meshing::mesh_renderer::ChunkMesh,
    rendering::materials::{CoreMaterial, DirtMaterial, GrassMaterial, RockMaterial},
    world::{
        chunk::{CHUNK_SIZE, Chunk},
        chunk_world::World,
        voxel::{Voxel, VoxelType},
    },
};

/// Voxel size in world units
pub const VOXEL_SIZE: f32 = 6.0;

pub struct VoxelRenderer;

impl VoxelRenderer {
    /// Render voxels as individual quads for each solid voxel
    pub fn render_voxels(
        chunk: &Chunk,
        chunk_x: i32,
        chunk_y: i32,
        material_type: VoxelType,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        rock_materials: &mut ResMut<Assets<RockMaterial>>,
        dirt_materials: &mut ResMut<Assets<DirtMaterial>>,
        grass_materials: &mut ResMut<Assets<GrassMaterial>>,
        core_materials: &mut ResMut<Assets<CoreMaterial>>,
    ) {
        let (vertices, indices) = Self::generate_voxel_mesh(chunk, chunk_x, chunk_y, material_type);

        if !vertices.is_empty() {
            let mesh = Self::create_mesh_from_vertices(vertices, indices);
            let mesh_handle = meshes.add(mesh);

            // Create appropriate material and spawn entity
            match material_type {
                VoxelType::Rock => {
                    let material = rock_materials.add(RockMaterial::default());
                    commands.spawn((
                        Mesh2d(mesh_handle),
                        MeshMaterial2d(material),
                        Transform::default(),
                        ChunkMesh { chunk_x, chunk_y },
                        Voxel,
                    ));
                }
                VoxelType::Dirt => {
                    let material = dirt_materials.add(DirtMaterial::default());
                    commands.spawn((
                        Mesh2d(mesh_handle),
                        MeshMaterial2d(material),
                        Transform::default(),
                        ChunkMesh { chunk_x, chunk_y },
                        Voxel,
                    ));
                }
                VoxelType::Grass => {
                    let material = grass_materials.add(GrassMaterial::default());
                    commands.spawn((
                        Mesh2d(mesh_handle),
                        MeshMaterial2d(material),
                        Transform::default(),
                        ChunkMesh { chunk_x, chunk_y },
                        Voxel,
                    ));
                }
                VoxelType::Core => {
                    let material = core_materials.add(CoreMaterial::default());
                    commands.spawn((
                        Mesh2d(mesh_handle),
                        MeshMaterial2d(material),
                        Transform::default(),
                        ChunkMesh { chunk_x, chunk_y },
                        Voxel,
                    ));
                }
                VoxelType::Air => {} // Skip air voxels
            }
        }
    }

    /// Generate mesh data for voxels of a specific material type in a chunk
    fn generate_voxel_mesh(
        chunk: &Chunk,
        chunk_x: i32,
        chunk_y: i32,
        material_type: VoxelType,
    ) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_count = 0u32;

        let chunk_world_x = chunk_x * CHUNK_SIZE as i32;
        let chunk_world_y = chunk_y * CHUNK_SIZE as i32;

        for dy in 0..CHUNK_SIZE {
            for dx in 0..CHUNK_SIZE {
                let voxel = chunk.get_voxel(dx, dy);

                // Only render voxels of the requested material type
                if voxel == material_type && voxel != VoxelType::Air {
                    let world_x = (chunk_world_x + dx as i32) as f32 * VOXEL_SIZE;
                    let world_y = (chunk_world_y + dy as i32) as f32 * VOXEL_SIZE;

                    // Create a quad for this voxel
                    let half_size = VOXEL_SIZE * 0.5;

                    // Add vertices for the quad (2D rectangle)
                    vertices.push([world_x - half_size, world_y - half_size, 0.0]); // Bottom-left
                    vertices.push([world_x + half_size, world_y - half_size, 0.0]); // Bottom-right
                    vertices.push([world_x + half_size, world_y + half_size, 0.0]); // Top-right
                    vertices.push([world_x - half_size, world_y + half_size, 0.0]); // Top-left

                    // Add indices for two triangles forming the quad
                    indices.extend_from_slice(&[
                        vertex_count,
                        vertex_count + 1,
                        vertex_count + 2, // First triangle
                        vertex_count,
                        vertex_count + 2,
                        vertex_count + 3, // Second triangle
                    ]);

                    vertex_count += 4;
                }
            }
        }

        (vertices, indices)
    }

    /// Create a Bevy mesh from vertices and indices
    fn create_mesh_from_vertices(vertices: Vec<[f32; 3]>, indices: Vec<u32>) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );

        // Set vertices
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone());

        // Generate normals (all pointing forward for 2D)
        let normals: Vec<[f32; 3]> = vertices.iter().map(|_| [0.0, 0.0, 1.0]).collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

        // Generate UV coordinates (simple mapping)
        let uvs: Vec<[f32; 2]> = vertices
            .iter()
            .enumerate()
            .map(|(i, _)| {
                match i % 4 {
                    0 => [0.0, 0.0], // Bottom-left
                    1 => [1.0, 0.0], // Bottom-right
                    2 => [1.0, 1.0], // Top-right
                    3 => [0.0, 1.0], // Top-left
                    _ => [0.0, 0.0],
                }
            })
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

        // Set indices
        mesh.insert_indices(Indices::U32(indices));

        mesh
    }

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
}
