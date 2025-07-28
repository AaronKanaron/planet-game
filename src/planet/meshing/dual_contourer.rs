use bevy::prelude::*;

use crate::planet::{
    meshing::mesh_renderer::VOXEL_SIZE,
    world::{chunk_world::World, voxel::VoxelType},
};

/// Dual contouring algorithm for generating meshes from voxel data
pub struct DualContourer;

#[derive(Debug)]
pub struct CellMesh {
    pub vertices: Vec<Vec2>,
    pub triangles: Vec<[u32; 3]>,
}

impl DualContourer {
    pub fn generate_chunk_mesh(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        target_type: VoxelType,
    ) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let chunk_size = World::chunk_size() as i32;

        // Calculate world bounds for this specific chunk
        let world_min_x = chunk_x * chunk_size;
        let world_max_x = (chunk_x + 1) * chunk_size;
        let world_min_y = chunk_y * chunk_size;
        let world_max_y = (chunk_y + 1) * chunk_size;

        // Generate dual contour mesh data for this chunk
        let mut complex_meshes = Vec::new();

        // Generate cells for this chunk, including boundary cells for seamless connection
        // We need to generate cells for the boundaries to ensure seamless edges
        for world_y in world_min_y..world_max_y {
            for world_x in world_min_x..world_max_x {
                // Check if this cell "belongs" to this chunk to avoid duplicates
                // A cell belongs to the chunk containing its bottom-left corner
                let cell_chunk_x = world_x.div_euclid(chunk_size);
                let cell_chunk_y = world_y.div_euclid(chunk_size);

                // Only process cells that belong to this chunk
                if cell_chunk_x == chunk_x && cell_chunk_y == chunk_y {
                    // For SDF-based terrain, skip the old discrete voxel approach
                    // Instead, directly generate smooth surface mesh using SDF sampling
                    if let Some(cell_mesh) = Self::generate_sdf_cell_mesh(world, world_x, world_y, target_type) {
                        complex_meshes.push(cell_mesh);
                    }
                }
            }
        }

        // Disable greedy meshing for SDF-based terrain to preserve smooth surfaces
        // let greedy_quads = GreedyMeshHandler::greedy_mesh_quads(
        //     &mesh_quads,
        //     world_min_x,
        //     world_min_y,
        //     world_max_x,
        //     world_max_y,
        // );

        // Add greedy meshed quads to the final mesh (disabled for SDF)
        // for quad in greedy_quads {
        //     GreedyMeshHandler::add_quad_to_mesh(&mut vertices, &mut indices, quad);
        // }

        // Add complex meshes to the final mesh
        for cell_mesh in complex_meshes {
            let vertex_offset = vertices.len() as u32;

            // Add vertices with proper world positioning
            for vertex in cell_mesh.vertices {
                vertices.push([vertex.x * VOXEL_SIZE, vertex.y * VOXEL_SIZE, 0.0]);
            }

            // Add indices with offset
            for triangle in cell_mesh.triangles {
                indices.push(vertex_offset + triangle[0]);
                indices.push(vertex_offset + triangle[1]);
                indices.push(vertex_offset + triangle[2]);
            }
        }

        (vertices, indices)
    }

    // v--------- Private methods ----------v //
    /// Generate mesh for a cell using proper SDF-based dual contouring
    fn generate_sdf_cell_mesh(
        world: &World,
        cell_x: i32,
        cell_y: i32,
        target_type: VoxelType,
    ) -> Option<CellMesh> {
        // Sample SDF at cell corners
        let corners = [
            Self::sample_sdf_at_point(world, cell_x as f32, cell_y as f32, target_type),
            Self::sample_sdf_at_point(world, cell_x as f32 + 1.0, cell_y as f32, target_type),
            Self::sample_sdf_at_point(world, cell_x as f32 + 1.0, cell_y as f32 + 1.0, target_type),
            Self::sample_sdf_at_point(world, cell_x as f32, cell_y as f32 + 1.0, target_type),
        ];

        // Check if this cell contains the surface
        let inside_corners: Vec<bool> = corners.iter().map(|&c| c <= 0.0).collect();
        let inside_count = inside_corners.iter().filter(|&&inside| inside).count();
        
        match inside_count {
            0 => None, // All outside - no surface
            4 => {
                // All inside - generate filled quad for solid material
                Some(Self::create_filled_quad(cell_x as f32, cell_y as f32))
            }
            _ => {
                // Surface crosses cell - use true dual contouring
                Self::generate_dual_contour_mesh(world, &corners, cell_x, cell_y, target_type)
            }
        }
    }

    /// Sample SDF at a specific point with proper material handling
    fn sample_sdf_at_point(world: &World, x: f32, y: f32, target_type: VoxelType) -> f32 {
        let ix = x.round() as i32;
        let iy = y.round() as i32;
        
        let chunk_size = World::chunk_size() as i32;
        let chunk_x = ix.div_euclid(chunk_size);
        let chunk_y = iy.div_euclid(chunk_size);

        if world.is_chunk_loaded(chunk_x, chunk_y) {
            let sdf = world.get_voxel_sdf(ix, iy);
            let actual_material = sdf.get_material();
            
            // For target material, return negative distance (inside)
            // For non-target material, return positive distance (outside)
            if actual_material == target_type {
                -sdf.distance.abs()
            } else {
                sdf.distance.abs()
            }
        } else {
            // Default for unloaded chunks - assume outside
            100.0
        }
    }

    /// Create a filled quad for completely solid cells
    fn create_filled_quad(cell_x: f32, cell_y: f32) -> CellMesh {
        CellMesh {
            vertices: vec![
                Vec2::new(cell_x, cell_y),           // bottom-left
                Vec2::new(cell_x + 1.0, cell_y),     // bottom-right
                Vec2::new(cell_x + 1.0, cell_y + 1.0), // top-right
                Vec2::new(cell_x, cell_y + 1.0),     // top-left
            ],
            triangles: vec![
                [0, 1, 2], // First triangle
                [0, 2, 3], // Second triangle
            ],
        }
    }

    /// Generate mesh using true dual contouring - finds optimal vertex per cell
    fn generate_dual_contour_mesh(
        world: &World,
        corners: &[f32; 4],
        cell_x: i32,
        cell_y: i32,
        target_type: VoxelType,
    ) -> Option<CellMesh> {
        // Find the optimal vertex position within this cell using dual contouring
        let vertex_pos = Self::find_dual_contour_vertex(corners, cell_x as f32, cell_y as f32);
        
        // For 2D dual contouring, we need to connect to neighboring cells
        // This is more complex than marching squares - we create quads between cell vertices
        
        let mut vertices = vec![vertex_pos];
        let mut triangles = Vec::new();
        
        // Check which edges have surface crossings and connect to neighboring cells
        let edge_crossings = Self::find_edge_crossings(corners);
        
        if edge_crossings.is_empty() {
            return None; // No surface crossings
        }
        
        // For each edge crossing, we need to connect to the adjacent cell's vertex
        // This creates the characteristic smooth surfaces of dual contouring
        let neighbor_connections = Self::get_neighbor_vertices(
            world, cell_x, cell_y, target_type, &edge_crossings
        );
        
        if neighbor_connections.len() < 2 {
            return None; // Need at least 2 connections for a surface
        }
        
        // Add neighbor vertices
        for neighbor_vertex in &neighbor_connections {
            vertices.push(*neighbor_vertex);
        }
        
        // Create triangles connecting the vertices
        // In dual contouring, we typically create triangle strips along surface flows
        if vertices.len() >= 3 {
            // Create triangles in a fan pattern, but optimized for dual contouring
            let center_idx = 0; // Our cell's vertex
            for i in 1..vertices.len() - 1 {
                triangles.push([center_idx, i as u32, (i + 1) as u32]);
            }
            
            // Close the fan if we have enough vertices
            if vertices.len() > 3 {
                triangles.push([center_idx, (vertices.len() - 1) as u32, 1]);
            }
        }
        
        if triangles.is_empty() {
            return None;
        }
        
        Some(CellMesh { vertices, triangles })
    }
    
    /// Find the optimal vertex position within a cell for dual contouring
    fn find_dual_contour_vertex(corners: &[f32; 4], cell_x: f32, cell_y: f32) -> Vec2 {
        // Calculate edge intersections using linear interpolation
        let intersections = Self::calculate_sdf_intersections(corners, cell_x, cell_y);
        
        if intersections.is_empty() {
            // Fallback to cell center if no intersections found
            return Vec2::new(cell_x + 0.5, cell_y + 0.5);
        }
        
        // For proper dual contouring, we should solve a least-squares problem to find
        // the vertex that minimizes distance to all the surface constraints.
        // For simplicity, we'll use the centroid of intersection points as approximation
        let mut centroid = Vec2::ZERO;
        for intersection in &intersections {
            centroid += *intersection;
        }
        centroid / intersections.len() as f32
    }
    
    /// Find which edges have surface crossings (sign changes)
    fn find_edge_crossings(corners: &[f32; 4]) -> Vec<usize> {
        let mut crossings = Vec::new();
        
        for i in 0..4 {
            let next = (i + 1) % 4;
            let sdf1 = corners[i];
            let sdf2 = corners[next];
            
            // Check for sign change (surface crossing)
            if (sdf1 > 0.0) != (sdf2 > 0.0) {
                crossings.push(i);
            }
        }
        
        crossings
    }
    
    /// Get vertices from neighboring cells that we should connect to
    fn get_neighbor_vertices(
        world: &World,
        cell_x: i32,
        cell_y: i32,
        target_type: VoxelType,
        edge_crossings: &[usize],
    ) -> Vec<Vec2> {
        let mut neighbor_vertices = Vec::new();
        
        // For each edge that has a crossing, check the adjacent cell
        for &edge_idx in edge_crossings {
            let (neighbor_x, neighbor_y) = match edge_idx {
                0 => (cell_x, cell_y - 1), // Bottom edge -> cell below
                1 => (cell_x + 1, cell_y), // Right edge -> cell to right  
                2 => (cell_x, cell_y + 1), // Top edge -> cell above
                3 => (cell_x - 1, cell_y), // Left edge -> cell to left
                _ => continue,
            };
            
            // Sample the neighbor cell and get its dual contour vertex
            if let Some(neighbor_vertex) = Self::get_neighbor_cell_vertex(
                world, neighbor_x, neighbor_y, target_type
            ) {
                neighbor_vertices.push(neighbor_vertex);
            }
        }
        
        neighbor_vertices
    }
    
    /// Get the dual contour vertex from a neighboring cell
    fn get_neighbor_cell_vertex(
        world: &World,
        neighbor_x: i32,
        neighbor_y: i32,
        target_type: VoxelType,
    ) -> Option<Vec2> {
        // Sample neighboring cell corners
        let neighbor_corners = [
            Self::sample_sdf_at_point(world, neighbor_x as f32, neighbor_y as f32, target_type),
            Self::sample_sdf_at_point(world, neighbor_x as f32 + 1.0, neighbor_y as f32, target_type),
            Self::sample_sdf_at_point(world, neighbor_x as f32 + 1.0, neighbor_y as f32 + 1.0, target_type),
            Self::sample_sdf_at_point(world, neighbor_x as f32, neighbor_y as f32 + 1.0, target_type),
        ];
        
        // Check if neighbor has surface crossing
        let inside_count = neighbor_corners.iter().filter(|&&c| c <= 0.0).count();
        if inside_count == 0 || inside_count == 4 {
            return None; // No surface in neighbor
        }
        
        // Calculate neighbor's dual contour vertex
        Some(Self::find_dual_contour_vertex(&neighbor_corners, neighbor_x as f32, neighbor_y as f32))
    }

    /// Calculate intersection points using SDF linear interpolation
    fn calculate_sdf_intersections(corners: &[f32; 4], cell_x: f32, cell_y: f32) -> Vec<Vec2> {
        let mut intersections = Vec::new();

        let corner_positions = [
            Vec2::new(cell_x, cell_y),             // bottom-left
            Vec2::new(cell_x + 1.0, cell_y),       // bottom-right
            Vec2::new(cell_x + 1.0, cell_y + 1.0), // top-right
            Vec2::new(cell_x, cell_y + 1.0),       // top-left
        ];

        // Check each edge for zero-crossing
        for i in 0..4 {
            let next = (i + 1) % 4;
            let sdf1 = corners[i];
            let sdf2 = corners[next];

            // Check for sign change (surface crossing)
            if (sdf1 > 0.0) != (sdf2 > 0.0) {
                // Linear interpolation to find exact crossing point
                let t = sdf1 / (sdf1 - sdf2);
                let intersection = corner_positions[i].lerp(corner_positions[next], t);
                intersections.push(intersection);
            }
        }

        intersections
    }
}
