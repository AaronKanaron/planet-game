use crate::planet::{
    world::voxel::VOXEL_SIZE,
    world::{chunk::CHUNK_SIZE, chunk_world::World},
};
use bevy::{platform::collections::HashMap, prelude::*};

// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

/// Represents a contour cell with an interior vertex for dual contouring
#[derive(Debug, Clone)]
pub struct ContourCell {
    pub x: usize,
    pub y: usize,
    pub corner_values: [bool; 4], // Corner solid/empty states
    pub interior_vertex: Vec2,    // Optimally placed vertex within cell
    pub normal: Vec2,             // Surface normal at vertex
}

/// Intersection data for an edge where surface crosses
#[derive(Debug, Clone)]
pub struct EdgeIntersection {
    pub position: Vec2, // Local position within cell (0.0-1.0)
    pub normal: Vec2,   // Surface normal at intersection
}

// =============================================================================
// SHARED VERTEX SYSTEM
// =============================================================================

/// Fixed-point coordinate for consistent vertex positioning across chunks
/// Uses integer coordinates (multiplied by 1000) for HashMap keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SnappedCoord {
    pub x: i32,
    pub y: i32,
}

impl SnappedCoord {
    pub fn from_world_pos(pos: Vec2) -> Self {
        Self {
            x: (pos.x * 1000.0) as i32,
            y: (pos.y * 1000.0) as i32,
        }
    }
}

/// A vertex on a chunk border that can be shared between chunks
/// Only stores the vertex ID since that's all that's actually used
#[derive(Debug, Clone)]
pub struct ChunkBorderVertex {
    pub vertex_id: usize,
}

/// Registry for managing shared vertices across chunk boundaries
#[derive(Debug, Default)]
pub struct SharedVertexRegistry {
    vertices: HashMap<SnappedCoord, ChunkBorderVertex>,
    next_vertex_id: usize,
}

impl SharedVertexRegistry {
    pub fn new() -> Self {
        Self {
            vertices: HashMap::new(),
            next_vertex_id: 0,
        }
    }

    /// Get or create a shared vertex at the given snapped coordinate
    pub fn get_or_create_vertex(&mut self, coord: SnappedCoord) -> usize {
        if let Some(vertex) = self.vertices.get(&coord) {
            vertex.vertex_id
        } else {
            let vertex_id = self.next_vertex_id;
            self.next_vertex_id += 1;

            self.vertices.insert(coord, ChunkBorderVertex { vertex_id });

            vertex_id
        }
    }

    /// Get all registered vertices for debugging purposes
    pub fn get_all_vertices(&self) -> impl Iterator<Item = (&SnappedCoord, &ChunkBorderVertex)> {
        self.vertices.iter()
    }
}

/// Information about an intersection on a chunk border
/// Only keeps world_position since that's what's actually used for debug visualization
#[derive(Debug, Clone)]
pub struct ChunkBorderIntersection {
    pub world_position: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkEdgeType {
    Left,   // x = 0
    Right,  // x = CHUNK_SIZE
    Bottom, // y = 0
    Top,    // y = CHUNK_SIZE
}

// =============================================================================
// DUAL CONTOURING IMPLEMENTATION
// =============================================================================

pub struct DualContouring;

impl DualContouring {
    // =============================================================================
    // PUBLIC API
    // =============================================================================

    /// Find contour cells with border intersection detection for shared vertex system
    ///
    /// This is the main entry point when you need cross-chunk vertex consistency.
    /// It returns both contour cells and border intersections that can be shared
    /// between adjacent chunks.
    pub fn find_contour_cells_with_borders(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        vertex_registry: &mut SharedVertexRegistry,
    ) -> (Vec<ContourCell>, Vec<ChunkBorderIntersection>) {
        let (contour_cells, border_intersections) =
            Self::process_chunk_cells(world, chunk_x, chunk_y, Some(vertex_registry));
        (contour_cells, border_intersections)
    }

    /// Find contour cells without border intersection detection (simpler version)
    ///
    /// Use this when you don't need cross-chunk vertex sharing. This is faster
    /// and simpler but may result in gaps between chunks.
    pub fn find_contour_cells(world: &World, chunk_x: i32, chunk_y: i32) -> Vec<ContourCell> {
        let (contour_cells, _) = Self::process_chunk_cells(world, chunk_x, chunk_y, None);
        contour_cells
    }

    /// Clear the shared vertex registry (useful when regenerating large areas)
    ///
    /// Call this when you want to completely regenerate a large area and don't
    /// want to preserve any existing shared vertices.
    pub fn clear_vertex_registry(world: &mut World) {
        let registry = world.get_vertex_registry_mut();
        registry.vertices.clear();
        registry.next_vertex_id = 0;
    }

    /// Process corner vertices for a chunk - public API for manual corner processing
    ///
    /// This can be called separately if you need to handle corner vertices
    /// without generating the full contour cells.
    pub fn process_chunk_corner_vertices(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        vertex_registry: &mut SharedVertexRegistry,
    ) {
        Self::process_corner_vertices(world, chunk_x, chunk_y, vertex_registry);
    }

    // =============================================================================
    // CORE PROCESSING
    // =============================================================================

    /// Process all cells in a chunk to find contour cells and optionally border intersections
    fn process_chunk_cells(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        mut vertex_registry: Option<&mut SharedVertexRegistry>,
    ) -> (Vec<ContourCell>, Vec<ChunkBorderIntersection>) {
        let mut contour_cells = Vec::new();
        let mut border_intersections = Vec::new();
        contour_cells.reserve(CHUNK_SIZE * CHUNK_SIZE / 4);

        // Use row-by-row processing to avoid redundant voxel lookups
        let mut prev_row = vec![None; CHUNK_SIZE + 1];
        let mut curr_row = vec![None; CHUNK_SIZE + 1];

        // Initialize first row
        for x in 0..=CHUNK_SIZE {
            prev_row[x] = Self::try_get_voxel(world, chunk_x, chunk_y, x as i32, 0);
        }

        // Process each row
        for y in 0..CHUNK_SIZE {
            // Load current row
            for x in 0..=CHUNK_SIZE {
                curr_row[x] =
                    Self::try_get_voxel(world, chunk_x, chunk_y, x as i32, (y + 1) as i32);
            }

            // Process each cell in the row
            for x in 0..CHUNK_SIZE {
                if let Some((corners, contour_cell)) =
                    Self::process_cell(world, chunk_x, chunk_y, x, y, &prev_row, &curr_row)
                {
                    contour_cells.push(contour_cell);

                    // Handle border intersections if vertex registry is provided
                    if let Some(ref mut registry) = vertex_registry {
                        let cell_border_intersections = Self::find_border_intersections(
                            world, chunk_x, chunk_y, x, y, &corners, registry,
                        );
                        border_intersections.extend(cell_border_intersections);
                    }
                }
            }

            std::mem::swap(&mut prev_row, &mut curr_row);
        }

        // Handle corner vertices if vertex registry is provided
        if let Some(ref mut registry) = vertex_registry {
            Self::process_corner_vertices(world, chunk_x, chunk_y, registry);
        }

        (contour_cells, border_intersections)
    }

    /// Process a single cell to determine if it contains a contour
    fn process_cell(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        x: usize,
        y: usize,
        prev_row: &[Option<bool>],
        curr_row: &[Option<bool>],
    ) -> Option<([bool; 4], ContourCell)> {
        let corner_opts = [prev_row[x], prev_row[x + 1], curr_row[x], curr_row[x + 1]];

        // Skip if any corner is in an unloaded chunk
        if corner_opts.iter().any(|opt| opt.is_none()) {
            return None;
        }

        let corners = [
            corner_opts[0].unwrap(),
            corner_opts[1].unwrap(),
            corner_opts[2].unwrap(),
            corner_opts[3].unwrap(),
        ];

        // Check if there's a sign change (surface crossing)
        let first_corner = corners[0];
        let has_sign_change =
            corners[1] != first_corner || corners[2] != first_corner || corners[3] != first_corner;

        if has_sign_change {
            let (interior_vertex, normal) =
                Self::solve_qef_for_cell(world, chunk_x, chunk_y, x, y, &corners);

            let contour_cell = ContourCell {
                x,
                y,
                corner_values: corners,
                interior_vertex,
                normal,
            };

            Some((corners, contour_cell))
        } else {
            None
        }
    }

    // =============================================================================
    // VOXEL SAMPLING
    // =============================================================================

    /// Get voxel state at given position, handling chunk boundaries
    /// Returns None if the voxel would be in an unloaded chunk
    fn try_get_voxel(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        local_x: i32,
        local_y: i32,
    ) -> Option<bool> {
        // Check if position is within current chunk
        if local_x >= 0
            && local_x < CHUNK_SIZE as i32
            && local_y >= 0
            && local_y < CHUNK_SIZE as i32
        {
            if let Some(chunk) = world.loaded_chunks.get(&(chunk_x, chunk_y)) {
                return Some(
                    chunk
                        .get_voxel(local_x as usize, local_y as usize)
                        .is_solid(),
                );
            }
        } else {
            // Handle neighbor chunk access
            let (neighbor_chunk_x, neighbor_local_x) =
                Self::calculate_neighbor_coords(chunk_x, local_x);
            let (neighbor_chunk_y, neighbor_local_y) =
                Self::calculate_neighbor_coords(chunk_y, local_y);

            if let Some(neighbor_chunk) = world
                .loaded_chunks
                .get(&(neighbor_chunk_x, neighbor_chunk_y))
            {
                return Some(
                    neighbor_chunk
                        .get_voxel(neighbor_local_x, neighbor_local_y)
                        .is_solid(),
                );
            }
        }

        None // Chunk not loaded
    }

    /// Calculate neighbor chunk coordinates and local position
    fn calculate_neighbor_coords(chunk_coord: i32, local_coord: i32) -> (i32, usize) {
        if local_coord < 0 {
            (chunk_coord - 1, (CHUNK_SIZE as i32 + local_coord) as usize)
        } else if local_coord >= CHUNK_SIZE as i32 {
            (chunk_coord + 1, (local_coord - CHUNK_SIZE as i32) as usize)
        } else {
            (chunk_coord, local_coord as usize)
        }
    }

    // =============================================================================
    // QEF SOLVING AND VERTEX PLACEMENT
    // =============================================================================

    /// Solve QEF (Quadratic Error Function) for optimal vertex placement in a cell
    fn solve_qef_for_cell(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        cell_x: usize,
        cell_y: usize,
        corners: &[bool; 4],
    ) -> (Vec2, Vec2) {
        let intersections =
            Self::find_edge_intersections(world, chunk_x, chunk_y, cell_x, cell_y, corners);

        if intersections.is_empty() {
            let cell_center = Vec2::new(
                (cell_x as f32 + 0.5) * VOXEL_SIZE,
                (cell_y as f32 + 0.5) * VOXEL_SIZE,
            );
            return (cell_center, Vec2::Y);
        }

        Self::solve_qef(&intersections, cell_x, cell_y)
    }

    /// Find intersections on cell edges where surface crosses
    fn find_edge_intersections(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        cell_x: usize,
        cell_y: usize,
        corners: &[bool; 4],
    ) -> Vec<EdgeIntersection> {
        let mut intersections = Vec::new();
        let edges = [(0, 1), (1, 3), (3, 2), (2, 0)]; // Edge connectivity (corner indices)

        for (edge_idx, (corner1_idx, corner2_idx)) in edges.iter().enumerate() {
            let corner1_solid = corners[*corner1_idx];
            let corner2_solid = corners[*corner2_idx];

            // Check if this edge has a sign change (surface crossing)
            if corner1_solid != corner2_solid {
                let intersection = Self::calculate_edge_intersection(
                    world,
                    chunk_x,
                    chunk_y,
                    cell_x,
                    cell_y,
                    edge_idx,
                    *corner1_idx,
                    *corner2_idx,
                );
                intersections.push(intersection);
            }
        }

        intersections
    }

    /// Calculate intersection point and normal for a specific edge
    fn calculate_edge_intersection(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        cell_x: usize,
        cell_y: usize,
        edge_idx: usize,
        _corner1_idx: usize,
        _corner2_idx: usize,
    ) -> EdgeIntersection {
        // Local position on edge (within cell, 0.0-1.0 range)
        let local_pos = match edge_idx {
            0 => Vec2::new(0.5, 0.0), // Bottom edge
            1 => Vec2::new(1.0, 0.5), // Right edge
            2 => Vec2::new(0.5, 1.0), // Top edge
            3 => Vec2::new(0.0, 0.5), // Left edge
            _ => unreachable!(),
        };

        let world_pos = Vec2::new(cell_x as f32 + local_pos.x, cell_y as f32 + local_pos.y);
        let normal = Self::calculate_sdf_gradient(world, chunk_x, chunk_y, world_pos);

        EdgeIntersection {
            position: local_pos,
            normal,
        }
    }

    /// Solve Quadratic Error Function to find optimal vertex placement
    fn solve_qef(
        intersections: &[EdgeIntersection],
        _cell_x: usize,
        _cell_y: usize,
    ) -> (Vec2, Vec2) {
        if intersections.is_empty() {
            return (Vec2::new(0.5, 0.5), Vec2::Y);
        }

        // Initial vertex position (weighted average of intersection positions)
        let mut vertex_pos = Vec2::ZERO;
        let mut total_weight = 0.0;

        for intersection in intersections {
            let weight = 1.0;
            vertex_pos += intersection.position * weight;
            total_weight += weight;
        }

        vertex_pos = if total_weight > 0.0 {
            vertex_pos / total_weight
        } else {
            Vec2::new(0.5, 0.5)
        };

        // Iterative refinement to minimize error
        for _iteration in 0..3 {
            let mut correction = Vec2::ZERO;
            let mut correction_weight = 0.0;

            for intersection in intersections {
                let to_intersection = intersection.position - vertex_pos;
                let distance_along_normal = to_intersection.dot(intersection.normal);
                let constraint_correction = intersection.normal * distance_along_normal * 0.3; // Damping

                correction += constraint_correction;
                correction_weight += 1.0;
            }

            if correction_weight > 0.0 {
                vertex_pos += correction / correction_weight;
            }
        }

        // Calculate weighted average normal
        let mut avg_normal = Vec2::ZERO;
        let mut normal_weight = 0.0;

        for intersection in intersections {
            let distance = (intersection.position - vertex_pos).length();
            let weight = 1.0 / (1.0 + distance * 2.0);

            avg_normal += intersection.normal * weight;
            normal_weight += weight;
        }

        let final_normal = if normal_weight > 0.001 && avg_normal.length() > 0.001 {
            avg_normal.normalize()
        } else {
            Vec2::Y
        };

        // Constrain vertex position to stay within cell bounds
        let margin = 0.1;
        let constrained_pos = Vec2::new(
            vertex_pos.x.clamp(margin, 1.0 - margin),
            vertex_pos.y.clamp(margin, 1.0 - margin),
        );

        // Apply smooth constraints for edge cases
        let final_pos = Self::apply_smooth_constraints(constrained_pos, margin);

        (final_pos, final_normal)
    }

    /// Apply smooth constraints to vertex position
    fn apply_smooth_constraints(pos: Vec2, margin: f32) -> Vec2 {
        let smooth_coordinate = |coord: f32| -> f32 {
            if coord < 0.2 || coord > 0.8 {
                let t = if coord < 0.5 {
                    coord * 5.0
                } else {
                    (1.0 - coord) * 5.0
                };
                let smooth_t = t * t * (3.0 - 2.0 * t); // Hermite interpolation
                if coord < 0.5 {
                    smooth_t * 0.2 + margin
                } else {
                    1.0 - (smooth_t * 0.2 + margin)
                }
            } else {
                coord
            }
        };

        Vec2::new(smooth_coordinate(pos.x), smooth_coordinate(pos.y))
    }

    // =============================================================================
    // SDF (SIGNED DISTANCE FIELD) FUNCTIONS
    // =============================================================================

    /// Calculate SDF gradient at a specific world position for proper normal computation
    fn calculate_sdf_gradient(world: &World, chunk_x: i32, chunk_y: i32, world_pos: Vec2) -> Vec2 {
        let epsilon = 0.05;

        // Sample SDF at 8 surrounding points for gradient calculation
        let samples = [
            Self::sample_sdf(world, chunk_x, chunk_y, world_pos.x - epsilon, world_pos.y),
            Self::sample_sdf(world, chunk_x, chunk_y, world_pos.x + epsilon, world_pos.y),
            Self::sample_sdf(world, chunk_x, chunk_y, world_pos.x, world_pos.y - epsilon),
            Self::sample_sdf(world, chunk_x, chunk_y, world_pos.x, world_pos.y + epsilon),
            Self::sample_sdf(
                world,
                chunk_x,
                chunk_y,
                world_pos.x - epsilon,
                world_pos.y - epsilon,
            ),
            Self::sample_sdf(
                world,
                chunk_x,
                chunk_y,
                world_pos.x + epsilon,
                world_pos.y - epsilon,
            ),
            Self::sample_sdf(
                world,
                chunk_x,
                chunk_y,
                world_pos.x - epsilon,
                world_pos.y + epsilon,
            ),
            Self::sample_sdf(
                world,
                chunk_x,
                chunk_y,
                world_pos.x + epsilon,
                world_pos.y + epsilon,
            ),
        ];

        // Calculate main gradients
        let dx_main = (samples[1] - samples[0]) / (2.0 * epsilon);
        let dy_main = (samples[3] - samples[2]) / (2.0 * epsilon);

        // Calculate diagonal gradients for better accuracy
        let dx_diag = ((samples[5] + samples[7]) - (samples[4] + samples[6])) / (4.0 * epsilon);
        let dy_diag = ((samples[6] + samples[7]) - (samples[4] + samples[5])) / (4.0 * epsilon);

        // Combine main and diagonal gradients
        let dx = dx_main * 0.7 + dx_diag * 0.3;
        let dy = dy_main * 0.7 + dy_diag * 0.3;

        let gradient = Vec2::new(dx, dy);

        if gradient.length() > 0.001 {
            gradient.normalize()
        } else {
            Vec2::Y // Default normal if gradient is too small
        }
    }

    /// Sample the Signed Distance Field at a specific world position
    /// Uses smoothed sampling for better continuity
    fn sample_sdf(world: &World, chunk_x: i32, chunk_y: i32, world_x: f32, world_y: f32) -> f32 {
        let voxel_x = world_x.floor() as i32;
        let voxel_y = world_y.floor() as i32;

        let fx = world_x - voxel_x as f32;
        let fy = world_y - voxel_y as f32;

        // Smooth interpolation factors
        // * * Used for the fallback bilinear sampling
        // let smooth_fx = fx * fx * (3.0 - 2.0 * fx);
        // let smooth_fy = fy * fy * (3.0 - 2.0 * fy);

        // Weighted sampling for smoother results
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for dy in -1..=1 {
            for dx in -1..=1 {
                let sample_x = voxel_x + dx;
                let sample_y = voxel_y + dy;

                let voxel_value = Self::try_get_voxel(world, chunk_x, chunk_y, sample_x, sample_y)
                    .map(|solid| if solid { -1.0 } else { 1.0 })
                    .unwrap_or(0.0);

                let dist_x = (dx as f32 - fx).abs();
                let dist_y = (dy as f32 - fy).abs();
                let distance = (dist_x * dist_x + dist_y * dist_y).sqrt();

                let weight = (-distance * distance * 2.0).exp();

                weighted_sum += voxel_value * weight;
                total_weight += weight;
            }
        }

        if total_weight > 0.001 {
            weighted_sum / total_weight
        } else {
            error!("Failed to sample SDF: total_weight is too low");
            // Self::bilinear_sdf_sample(
            //     world, chunk_x, chunk_y, voxel_x, voxel_y, smooth_fx, smooth_fy,
            // )
            0.0
        }
    }

    //? Unsure if this fallback is ever called. Keep if error comes up in future.
    /// Fallback bilinear SDF sampling
    // fn bilinear_sdf_sample(
    //     world: &World,
    //     chunk_x: i32,
    //     chunk_y: i32,
    //     voxel_x: i32,
    //     voxel_y: i32,
    //     fx: f32,
    //     fy: f32,
    // ) -> f32 {
    //     let v00 = Self::try_get_voxel(world, chunk_x, chunk_y, voxel_x, voxel_y)
    //         .map(|solid| if solid { -1.0 } else { 1.0 })
    //         .unwrap_or(0.0);
    //     let v10 = Self::try_get_voxel(world, chunk_x, chunk_y, voxel_x + 1, voxel_y)
    //         .map(|solid| if solid { -1.0 } else { 1.0 })
    //         .unwrap_or(0.0);
    //     let v01 = Self::try_get_voxel(world, chunk_x, chunk_y, voxel_x, voxel_y + 1)
    //         .map(|solid| if solid { -1.0 } else { 1.0 })
    //         .unwrap_or(0.0);
    //     let v11 = Self::try_get_voxel(world, chunk_x, chunk_y, voxel_x + 1, voxel_y + 1)
    //         .map(|solid| if solid { -1.0 } else { 1.0 })
    //         .unwrap_or(0.0);

    //     let v0 = v00 * (1.0 - fx) + v10 * fx;
    //     let v1 = v01 * (1.0 - fx) + v11 * fx;
    //     v0 * (1.0 - fy) + v1 * fy
    // }

    // =============================================================================
    // BORDER INTERSECTION HANDLING
    // =============================================================================

    /// Process corner vertices for a chunk - adds vertices at chunk corners when inside solid material
    /// This ensures proper meshing at chunk boundaries where corners are solid
    fn process_corner_vertices(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        vertex_registry: &mut SharedVertexRegistry,
    ) {
        // Define the 4 corner positions in local chunk coordinates
        let corner_positions = [
            (0, 0),                                 // Bottom-left
            (CHUNK_SIZE as i32, 0),                 // Bottom-right
            (0, CHUNK_SIZE as i32),                 // Top-left
            (CHUNK_SIZE as i32, CHUNK_SIZE as i32), // Top-right
        ];

        for (local_x, local_y) in corner_positions.iter() {
            // Sample voxel at the exact corner coordinate
            if let Some(is_solid) = Self::try_get_voxel(world, chunk_x, chunk_y, *local_x, *local_y)
            {
                // If corner is inside solid material, register it as a vertex
                if is_solid {
                    // Convert local coordinates to world coordinates
                    let chunk_world_x = chunk_x as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE;
                    let chunk_world_y = chunk_y as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE;
                    let corner_world_x = chunk_world_x + (*local_x as f32 * VOXEL_SIZE);
                    let corner_world_y = chunk_world_y + (*local_y as f32 * VOXEL_SIZE);

                    let world_pos = Vec2::new(corner_world_x, corner_world_y);
                    let snapped_coord = SnappedCoord::from_world_pos(world_pos);

                    // Register the corner vertex in the shared system
                    // Adjacent chunks will generate the same corner vertex automatically
                    let _vertex_id = vertex_registry.get_or_create_vertex(snapped_coord);
                }
            }
        }
    }

    /// Find border intersections for the given cell and register them in the shared vertex system
    fn find_border_intersections(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        cell_x: usize,
        cell_y: usize,
        corners: &[bool; 4],
        vertex_registry: &mut SharedVertexRegistry,
    ) -> Vec<ChunkBorderIntersection> {
        let mut border_intersections = Vec::new();
        let edges = [(0, 1, 0), (1, 3, 1), (3, 2, 2), (2, 0, 3)]; // (corner1, corner2, edge_idx)

        for (corner1_idx, corner2_idx, edge_idx) in edges.iter() {
            let corner1_solid = corners[*corner1_idx];
            let corner2_solid = corners[*corner2_idx];

            // Check for surface crossing on this edge
            if corner1_solid != corner2_solid {
                if let Some(_border_type) = Self::get_border_type(cell_x, cell_y, *edge_idx) {
                    let intersection = Self::calculate_edge_intersection(
                        world,
                        chunk_x,
                        chunk_y,
                        cell_x,
                        cell_y,
                        *edge_idx,
                        *corner1_idx,
                        *corner2_idx,
                    );

                    // Convert to world coordinates
                    let chunk_world_x = chunk_x as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE;
                    let chunk_world_y = chunk_y as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE;
                    let cell_world_x = chunk_world_x + cell_x as f32 * VOXEL_SIZE;
                    let cell_world_y = chunk_world_y + cell_y as f32 * VOXEL_SIZE;

                    let world_pos = Vec2::new(
                        cell_world_x + intersection.position.x * VOXEL_SIZE,
                        cell_world_y + intersection.position.y * VOXEL_SIZE,
                    );

                    let snapped_coord = SnappedCoord::from_world_pos(world_pos);

                    // Register vertex in shared system
                    let _vertex_id = vertex_registry.get_or_create_vertex(snapped_coord);

                    border_intersections.push(ChunkBorderIntersection {
                        world_position: world_pos,
                    });
                }
            }
        }

        border_intersections
    }

    /// Determine if an edge crosses a chunk boundary and return the border type
    fn get_border_type(cell_x: usize, cell_y: usize, edge_idx: usize) -> Option<ChunkEdgeType> {
        match edge_idx {
            0 => (cell_y == 0).then_some(ChunkEdgeType::Bottom),
            1 => (cell_x == CHUNK_SIZE - 1).then_some(ChunkEdgeType::Right),
            2 => (cell_y == CHUNK_SIZE - 1).then_some(ChunkEdgeType::Top),
            3 => (cell_x == 0).then_some(ChunkEdgeType::Left),
            _ => None,
        }
    }
}
