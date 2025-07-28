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

    /// Generate mesh using true dual contouring - creates smooth contour lines AND filled areas
    /// This generates both the smooth curved boundaries and fills the interior portions
    fn generate_dual_contour_mesh(
        world: &World,
        corners: &[f32; 4],
        cell_x: i32,
        cell_y: i32,
        target_type: VoxelType,
    ) -> Option<CellMesh> {
        // Check which edges have surface crossings
        let edge_crossings = Self::find_edge_crossings(corners);
        
        if edge_crossings.is_empty() {
            return None; // No surface crossings
        }

        // Find edge intersection points where the surface crosses cell boundaries
        let intersections = Self::calculate_sdf_intersections(corners, cell_x as f32, cell_y as f32);
        
        if intersections.len() < 2 {
            return None; // Need at least 2 intersections to create a surface
        }

        // STEP 1: Generate smooth contour lines (this is the key dual contouring feature)
        let contour_mesh = Self::generate_smooth_contour_lines(
            world, &intersections, cell_x, cell_y, target_type, &edge_crossings
        );

        // STEP 2: Generate filled area for the interior portion
        let fill_mesh = Self::generate_interior_fill(
            &intersections, corners, cell_x as f32, cell_y as f32
        );

        // STEP 3: Combine both the smooth contour lines and the fill area
        Some(Self::combine_contour_and_fill(contour_mesh, fill_mesh))
    }
    
    /// Generate smooth contour lines using proper dual contouring
    /// This creates the smooth curved boundaries that dual contouring is famous for
    fn generate_smooth_contour_lines(
        world: &World,
        intersections: &[Vec2],
        cell_x: i32,
        cell_y: i32,
        target_type: VoxelType,
        edge_crossings: &[usize],
    ) -> CellMesh {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();

        // Find the optimal dual contour vertex position within this cell
        let cell_vertex = Self::find_dual_contour_vertex(intersections, cell_x as f32, cell_y as f32);
        vertices.push(cell_vertex);

        // Get neighbor vertices for smooth contour connections
        let neighbor_vertices = Self::get_neighbor_vertices(
            world, cell_x, cell_y, target_type, edge_crossings
        );

        // Add neighbor vertices
        for neighbor_vertex in &neighbor_vertices {
            vertices.push(*neighbor_vertex);
        }

        // Create smooth contour lines by connecting to neighbors
        if vertices.len() >= 3 {
            // Create triangular strips that form smooth curves
            let center_idx = 0; // Our cell's vertex
            for i in 1..vertices.len() - 1 {
                triangles.push([center_idx, i as u32, (i + 1) as u32]);
            }
            
            // Close the contour if we have enough vertices
            if vertices.len() > 3 {
                triangles.push([center_idx, (vertices.len() - 1) as u32, 1]);
            }
        }

        CellMesh { vertices, triangles }
    }

    /// Generate filled area for the interior portion of boundary cells
    /// This fills the space between contour curves and inside corners
    fn generate_interior_fill(
        intersections: &[Vec2],
        corners: &[f32; 4],
        cell_x: f32,
        cell_y: f32,
    ) -> CellMesh {
        // Debug output for problematic cells
        let is_debug_cell = Self::is_debug_worthy_cell(intersections, corners);
        
        if is_debug_cell {
            println!("DEBUG CELL at ({}, {})", cell_x, cell_y);
            println!("  Corners SDF: {:?}", corners);
            println!("  Intersections: {:?}", intersections);
        }
        
        // Build polygon more carefully for complex boundary cases
        let polygon_vertices = Self::build_boundary_polygon(
            intersections, corners, cell_x, cell_y, is_debug_cell
        );
        
        if polygon_vertices.len() < 3 {
            return CellMesh {
                vertices: Vec::new(),
                triangles: Vec::new(),
            };
        }

        // Use proper triangulation for complex polygons instead of simple fan
        let triangles = Self::triangulate_polygon(&polygon_vertices, is_debug_cell);
        
        if is_debug_cell {
            println!("  Final vertices: {:?}", polygon_vertices);
            println!("  Triangles: {:?}", triangles);
        }

        CellMesh {
            vertices: polygon_vertices,
            triangles,
        }
    }

    /// Combine smooth contour lines with interior fill
    /// The key insight: contour lines provide smooth boundaries, fill provides complete coverage
    fn combine_contour_and_fill(contour: CellMesh, fill: CellMesh) -> CellMesh {
        // Both are important:
        // - Contour provides smooth curves between cells
        // - Fill provides complete interior coverage
        
        // If we have both, combine them properly
        if !contour.vertices.is_empty() && !fill.vertices.is_empty() {
            let mut combined_vertices = fill.vertices; // Start with fill
            let mut combined_triangles = fill.triangles;

            // Add contour vertices with offset for indices
            let vertex_offset = combined_vertices.len() as u32;
            combined_vertices.extend(contour.vertices);

            // Add contour triangles with corrected indices
            for triangle in contour.triangles {
                combined_triangles.push([
                    triangle[0] + vertex_offset,
                    triangle[1] + vertex_offset,
                    triangle[2] + vertex_offset,
                ]);
            }

            return CellMesh {
                vertices: combined_vertices,
                triangles: combined_triangles,
            };
        }
        
        // If only one is available, use it
        if !fill.vertices.is_empty() {
            return fill;
        }
        
        contour
    }

    /// Check if this cell is worth debugging (has complex boundary)
    fn is_debug_worthy_cell(intersections: &[Vec2], corners: &[f32; 4]) -> bool {
        // Only debug very specific problematic cells to reduce spam
        if intersections.len() >= 4 {
            return true; // Very complex boundaries
        }
        
        // Debug cells with very specific corner patterns that might cause issues
        let inside_count = corners.iter().filter(|&&c| c <= 0.0).count();
        if inside_count == 1 && intersections.len() >= 3 {
            return true; // Single inside corner with many intersections
        }
        
        false
    }

    /// Build a proper boundary polygon for complex cases
    fn build_boundary_polygon(
        intersections: &[Vec2],
        corners: &[f32; 4],
        cell_x: f32,
        cell_y: f32,
        debug: bool,
    ) -> Vec<Vec2> {
        let corner_positions = [
            Vec2::new(cell_x, cell_y),             // bottom-left (0)
            Vec2::new(cell_x + 1.0, cell_y),       // bottom-right (1)
            Vec2::new(cell_x + 1.0, cell_y + 1.0), // top-right (2)
            Vec2::new(cell_x, cell_y + 1.0),       // top-left (3)
        ];

        // Instead of just adding all points, build the polygon by walking the cell boundary
        let mut polygon_vertices = Vec::new();
        
        // Map intersections to their edge indices
        let mut edge_intersections: Vec<(usize, Vec2)> = Vec::new();
        for i in 0..4 {
            let next = (i + 1) % 4;
            let sdf1 = corners[i];
            let sdf2 = corners[next];
            
            // Check for surface crossing on this edge
            if (sdf1 > 0.0) != (sdf2 > 0.0) {
                // Find the intersection point on this edge
                let t = sdf1 / (sdf1 - sdf2);
                let intersection = corner_positions[i].lerp(corner_positions[next], t);
                edge_intersections.push((i, intersection));
            }
        }
        
        if debug {
            println!("  Edge intersections: {:?}", edge_intersections);
        }

        // Walk around the cell boundary and build the polygon properly
        for i in 0..4 {
            let corner_sdf = corners[i];
            
            // If this corner is inside, add it
            if corner_sdf <= 0.0 {
                polygon_vertices.push(corner_positions[i]);
                if debug {
                    println!("  Added inside corner {}: {:?}", i, corner_positions[i]);
                }
            }
            
            // Check if there's an intersection on the edge from this corner to the next
            if let Some((_, intersection)) = edge_intersections.iter().find(|(edge_idx, _)| *edge_idx == i) {
                polygon_vertices.push(*intersection);
                if debug {
                    println!("  Added intersection on edge {}: {:?}", i, intersection);
                }
            }
        }

        // Remove duplicate vertices that are too close together
        Self::remove_duplicate_vertices(polygon_vertices, 0.001)
    }

    /// Remove vertices that are too close together to avoid degenerate triangles
    fn remove_duplicate_vertices(vertices: Vec<Vec2>, tolerance: f32) -> Vec<Vec2> {
        if vertices.len() < 2 {
            return vertices;
        }
        
        let mut filtered = vec![vertices[0]];
        
        for vertex in vertices.iter().skip(1) {
            let last = filtered.last().unwrap();
            if (*vertex - *last).length() > tolerance {
                filtered.push(*vertex);
            }
        }
        
        // Check if first and last are too close
        if filtered.len() > 2 {
            let first = filtered[0];
            let last = *filtered.last().unwrap();
            if (first - last).length() <= tolerance {
                filtered.pop();
            }
        }
        
        filtered
    }

    /// Proper triangulation for complex polygons using ear clipping
    fn triangulate_polygon(vertices: &[Vec2], debug: bool) -> Vec<[u32; 3]> {
        if vertices.len() < 3 {
            return Vec::new();
        }
        
        if vertices.len() == 3 {
            return vec![[0, 1, 2]];
        }
        
        if vertices.len() == 4 {
            // For quads, check if it's convex and triangulate appropriately
            return Self::triangulate_quad(vertices, debug);
        }
        
        // For complex polygons, use ear clipping algorithm
        Self::ear_clipping_triangulation(vertices, debug)
    }

    /// Triangulate a quad properly
    fn triangulate_quad(vertices: &[Vec2], debug: bool) -> Vec<[u32; 3]> {
        // Check if the quad is convex by testing the cross product of consecutive edges
        let edge1 = vertices[1] - vertices[0];
        let edge2 = vertices[2] - vertices[1];
        let edge3 = vertices[3] - vertices[2];
        let edge4 = vertices[0] - vertices[3];
        
        let cross1 = edge1.x * edge2.y - edge1.y * edge2.x;
        let cross2 = edge2.x * edge3.y - edge2.y * edge3.x;
        let cross3 = edge3.x * edge4.y - edge3.y * edge4.x;
        let cross4 = edge4.x * edge1.y - edge4.y * edge1.x;
        
        let all_same_sign = (cross1 >= 0.0 && cross2 >= 0.0 && cross3 >= 0.0 && cross4 >= 0.0) ||
                           (cross1 <= 0.0 && cross2 <= 0.0 && cross3 <= 0.0 && cross4 <= 0.0);
        
        if debug {
            println!("  Quad cross products: {}, {}, {}, {}", cross1, cross2, cross3, cross4);
            println!("  Is convex: {}", all_same_sign);
        }
        
        if all_same_sign {
            // Convex quad - use standard triangulation
            vec![[0, 1, 2], [0, 2, 3]]
        } else {
            // Concave quad - need to find the correct diagonal
            // Try both diagonals and pick the one that doesn't create flipped triangles
            let area1 = Self::triangle_area(vertices[0], vertices[1], vertices[2]);
            let area2 = Self::triangle_area(vertices[0], vertices[2], vertices[3]);
            
            if area1 > 0.0 && area2 > 0.0 {
                vec![[0, 1, 2], [0, 2, 3]]
            } else {
                vec![[1, 2, 3], [1, 3, 0]]
            }
        }
    }

    /// Simple ear clipping triangulation for complex polygons
    fn ear_clipping_triangulation(vertices: &[Vec2], debug: bool) -> Vec<[u32; 3]> {
        if vertices.len() < 3 {
            return Vec::new();
        }
        
        let mut triangles = Vec::new();
        let mut remaining_indices: Vec<usize> = (0..vertices.len()).collect();
        
        if debug {
            println!("  Starting ear clipping with {} vertices", vertices.len());
        }
        
        // Keep removing ears until we have a triangle
        while remaining_indices.len() > 3 {
            let mut ear_found = false;
            
            for i in 0..remaining_indices.len() {
                let prev_idx = remaining_indices[(i + remaining_indices.len() - 1) % remaining_indices.len()];
                let curr_idx = remaining_indices[i];
                let next_idx = remaining_indices[(i + 1) % remaining_indices.len()];
                
                let v_prev = vertices[prev_idx];
                let v_curr = vertices[curr_idx];
                let v_next = vertices[next_idx];
                
                // Check if this forms an ear (convex vertex with no other vertices inside)
                if Self::is_ear(v_prev, v_curr, v_next, vertices, &remaining_indices) {
                    triangles.push([prev_idx as u32, curr_idx as u32, next_idx as u32]);
                    remaining_indices.remove(i);
                    ear_found = true;
                    
                    if debug {
                        println!("  Found ear: ({}, {}, {})", prev_idx, curr_idx, next_idx);
                    }
                    break;
                }
            }
            
            if !ear_found {
                if debug {
                    println!("  No ear found, falling back to fan triangulation");
                }
                // Fallback to fan triangulation if ear clipping fails
                break;
            }
        }
        
        // Add the final triangle
        if remaining_indices.len() == 3 {
            triangles.push([
                remaining_indices[0] as u32,
                remaining_indices[1] as u32,
                remaining_indices[2] as u32,
            ]);
        }
        
        triangles
    }

    /// Check if a vertex forms an ear
    fn is_ear(v_prev: Vec2, v_curr: Vec2, v_next: Vec2, all_vertices: &[Vec2], remaining_indices: &[usize]) -> bool {
        // Check if the angle is convex
        let edge1 = v_curr - v_prev;
        let edge2 = v_next - v_curr;
        let cross = edge1.x * edge2.y - edge1.y * edge2.x;
        
        if cross <= 0.0 {
            return false; // Concave vertex, not an ear
        }
        
        // Check if any other vertex is inside this triangle
        for &idx in remaining_indices {
            let test_vertex = all_vertices[idx];
            if test_vertex != v_prev && test_vertex != v_curr && test_vertex != v_next {
                if Self::point_in_triangle(test_vertex, v_prev, v_curr, v_next) {
                    return false; // Another vertex is inside, not an ear
                }
            }
        }
        
        true
    }

    /// Check if a point is inside a triangle
    fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
        let v0 = c - a;
        let v1 = b - a;
        let v2 = p - a;
        
        let dot00 = v0.dot(v0);
        let dot01 = v0.dot(v1);
        let dot02 = v0.dot(v2);
        let dot11 = v1.dot(v1);
        let dot12 = v1.dot(v2);
        
        let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
        let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
        
        (u >= 0.0) && (v >= 0.0) && (u + v <= 1.0)
    }

    /// Calculate triangle area (positive for counter-clockwise)
    fn triangle_area(a: Vec2, b: Vec2, c: Vec2) -> f32 {
        0.5 * ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y))
    }

    /// Find the optimal dual contour vertex position within a cell
    fn find_dual_contour_vertex(intersections: &[Vec2], cell_x: f32, cell_y: f32) -> Vec2 {
        if intersections.is_empty() {
            // Fallback to cell center if no intersections found
            return Vec2::new(cell_x + 0.5, cell_y + 0.5);
        }
        
        // For proper dual contouring, we should solve a least-squares problem to find
        // the vertex that minimizes distance to all the surface constraints.
        // For simplicity, we'll use the centroid of intersection points as approximation
        let mut centroid = Vec2::ZERO;
        for intersection in intersections {
            centroid += *intersection;
        }
        centroid / intersections.len() as f32
    }

    /// Get vertices from neighboring cells for smooth contour connections
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
        let neighbor_intersections = Self::calculate_sdf_intersections(&neighbor_corners, neighbor_x as f32, neighbor_y as f32);
        Some(Self::find_dual_contour_vertex(&neighbor_intersections, neighbor_x as f32, neighbor_y as f32))
    }

    /// Sort vertices to form a proper polygon (counter-clockwise order)
    fn sort_vertices_for_polygon(vertices: &[Vec2]) -> Vec<Vec2> {
        if vertices.len() < 3 {
            return vertices.to_vec();
        }

        // Calculate centroid
        let centroid = vertices.iter().fold(Vec2::ZERO, |acc, &v| acc + v) / vertices.len() as f32;
        
        // Sort vertices by angle from centroid
        let mut sorted_vertices: Vec<Vec2> = vertices.to_vec();
        sorted_vertices.sort_by(|&a, &b| {
            let angle_a = (a - centroid).y.atan2((a - centroid).x);
            let angle_b = (b - centroid).y.atan2((b - centroid).x);
            angle_a.partial_cmp(&angle_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        sorted_vertices
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
