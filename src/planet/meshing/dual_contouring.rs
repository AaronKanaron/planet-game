/* Imports */
use crate::planet::{
    meshing::render_voxels::VOXEL_SIZE,
    world::{chunk::CHUNK_SIZE, chunk_world::World},
};
use bevy::prelude::*;

/* Structs */
#[derive(Debug, Clone)]
pub struct ContourCell {
    pub x: usize,
    pub y: usize,
    pub corner_values: [bool; 4],
    pub interior_vertex: Vec2,
    pub normal: Vec2,
}

#[derive(Debug, Clone)]
pub struct EdgeIntersection {
    pub position: Vec2,
    pub normal: Vec2,
}

pub struct DualContouring;

/* Implementations */
impl DualContouring {
    pub fn find_contour_cells(world: &World, chunk_x: i32, chunk_y: i32) -> Vec<ContourCell> {
        let mut contour_cells = Vec::new();

        contour_cells.reserve(CHUNK_SIZE * CHUNK_SIZE / 4);

        let mut prev_row = vec![None; CHUNK_SIZE + 1];
        let mut curr_row = vec![None; CHUNK_SIZE + 1];

        for x in 0..=CHUNK_SIZE {
            prev_row[x] = Self::try_get_voxel(world, chunk_x, chunk_y, x as i32, 0);
        }

        for y in 0..CHUNK_SIZE {
            for x in 0..=CHUNK_SIZE {
                curr_row[x] =
                    Self::try_get_voxel(world, chunk_x, chunk_y, x as i32, (y + 1) as i32);
            }

            for x in 0..CHUNK_SIZE {
                let corner_opts = [
                    prev_row[x],     // bottom-left  (y, x)
                    prev_row[x + 1], // bottom-right (y, x+1)
                    curr_row[x],     // top-left     (y+1, x)
                    curr_row[x + 1], // top-right    (y+1, x+1)
                ];

                if corner_opts.iter().any(|opt| opt.is_none()) {
                    // skip if any corner is in an unloaded chunk
                    continue;
                }

                let corners = [
                    corner_opts[0].unwrap(),
                    corner_opts[1].unwrap(),
                    corner_opts[2].unwrap(),
                    corner_opts[3].unwrap(),
                ];

                let first_corner = corners[0];
                let has_sign_change = corners[1] != first_corner
                    || corners[2] != first_corner
                    || corners[3] != first_corner;

                if has_sign_change {
                    // Find edge intersections and calculate interior vertex
                    let (interior_vertex, normal) =
                        Self::solve_qef_for_cell(world, chunk_x, chunk_y, x, y, &corners);

                    contour_cells.push(ContourCell {
                        x,
                        y,
                        corner_values: corners,
                        interior_vertex,
                        normal,
                    });
                }
            }

            std::mem::swap(&mut prev_row, &mut curr_row);
        }

        contour_cells
    }

    /// Get if the voxel at the given position is solid
    /// Returns None if the voxel would be in an unloaded chunk
    fn try_get_voxel(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        local_x: i32,
        local_y: i32,
    ) -> Option<bool> {
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
            // we check the neighboring chunk
            let (neighbor_chunk_x, neighbor_local_x) = if local_x < 0 {
                (chunk_x - 1, (CHUNK_SIZE as i32 + local_x) as usize)
            } else if local_x >= CHUNK_SIZE as i32 {
                (chunk_x + 1, (local_x - CHUNK_SIZE as i32) as usize)
            } else {
                (chunk_x, local_x as usize)
            };

            let (neighbor_chunk_y, neighbor_local_y) = if local_y < 0 {
                (chunk_y - 1, (CHUNK_SIZE as i32 + local_y) as usize)
            } else if local_y >= CHUNK_SIZE as i32 {
                (chunk_y + 1, (local_y - CHUNK_SIZE as i32) as usize)
            } else {
                (chunk_y, local_y as usize)
            };

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

        // Return None if chunk not loaded
        None
    }

    /// Find edge intersections and solve QEF to place interior vertex
    fn solve_qef_for_cell(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        cell_x: usize,
        cell_y: usize,
        corners: &[bool; 4],
    ) -> (Vec2, Vec2) {
        // Find all edge intersections in this cell
        let intersections =
            Self::find_edge_intersections(world, chunk_x, chunk_y, cell_x, cell_y, corners);

        if intersections.is_empty() {
            // Fallback to cell center if no intersections found
            let cell_center = Vec2::new(
                (cell_x as f32 + 0.5) * VOXEL_SIZE,
                (cell_y as f32 + 0.5) * VOXEL_SIZE,
            );
            return (cell_center, Vec2::Y); // Default normal pointing up
        }

        // Solve QEF using the edge intersections
        Self::solve_qef(&intersections, cell_x, cell_y)
    }

    /// Find intersections on cell edges where sign changes occur
    fn find_edge_intersections(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        cell_x: usize,
        cell_y: usize,
        corners: &[bool; 4],
    ) -> Vec<EdgeIntersection> {
        let mut intersections = Vec::new();

        // Check each of the 4 edges of the cell
        // Edge indices: 0=bottom, 1=right, 2=top, 3=left
        let edges = [
            (0, 1), // bottom edge: bottom-left to bottom-right
            (1, 3), // right edge: bottom-right to top-right
            (3, 2), // top edge: top-right to top-left
            (2, 0), // left edge: top-left to bottom-left
        ];

        for (edge_idx, (corner1_idx, corner2_idx)) in edges.iter().enumerate() {
            let corner1_solid = corners[*corner1_idx];
            let corner2_solid = corners[*corner2_idx];

            // Check if this edge has a sign change
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
        // Calculate local position within the cell (0.0 to 1.0)
        let local_pos = match edge_idx {
            0 => Vec2::new(0.5, 0.0), // middle of bottom edge
            1 => Vec2::new(1.0, 0.5), // middle of right edge
            2 => Vec2::new(0.5, 1.0), // middle of top edge
            3 => Vec2::new(0.0, 0.5), // middle of left edge
            _ => unreachable!(),
        };

        // Calculate world position of the intersection for SDF sampling
        let world_pos = Vec2::new(
            (cell_x as f32 + local_pos.x) as f32,
            (cell_y as f32 + local_pos.y) as f32,
        );

        // Calculate proper SDF gradient at this intersection point
        let normal = Self::calculate_sdf_gradient(world, chunk_x, chunk_y, world_pos);

        EdgeIntersection {
            position: local_pos,
            normal,
        }
    }

    /// Calculate SDF gradient at a specific world position for proper normal computation
    /// Uses a larger sampling pattern for smoother gradients
    fn calculate_sdf_gradient(world: &World, chunk_x: i32, chunk_y: i32, world_pos: Vec2) -> Vec2 {
        // Use smaller epsilon for finer gradient estimation
        let epsilon = 0.05;

        // Sample SDF in a 3x3 pattern around the point for better gradient estimation
        let samples = [
            // Center cross pattern
            Self::sample_sdf(world, chunk_x, chunk_y, world_pos.x - epsilon, world_pos.y), // left
            Self::sample_sdf(world, chunk_x, chunk_y, world_pos.x + epsilon, world_pos.y), // right
            Self::sample_sdf(world, chunk_x, chunk_y, world_pos.x, world_pos.y - epsilon), // down
            Self::sample_sdf(world, chunk_x, chunk_y, world_pos.x, world_pos.y + epsilon), // up
            // Diagonal samples for better smoothing
            Self::sample_sdf(
                world,
                chunk_x,
                chunk_y,
                world_pos.x - epsilon,
                world_pos.y - epsilon,
            ), // bottom-left
            Self::sample_sdf(
                world,
                chunk_x,
                chunk_y,
                world_pos.x + epsilon,
                world_pos.y - epsilon,
            ), // bottom-right
            Self::sample_sdf(
                world,
                chunk_x,
                chunk_y,
                world_pos.x - epsilon,
                world_pos.y + epsilon,
            ), // top-left
            Self::sample_sdf(
                world,
                chunk_x,
                chunk_y,
                world_pos.x + epsilon,
                world_pos.y + epsilon,
            ), // top-right
        ];

        // Calculate gradient using weighted central differences with diagonal smoothing
        let dx_main = (samples[1] - samples[0]) / (2.0 * epsilon); // right - left
        let dy_main = (samples[3] - samples[2]) / (2.0 * epsilon); // up - down

        // Diagonal contributions (weighted less for smoothing)
        let dx_diag = ((samples[5] + samples[7]) - (samples[4] + samples[6])) / (4.0 * epsilon);
        let dy_diag = ((samples[6] + samples[7]) - (samples[4] + samples[5])) / (4.0 * epsilon);

        // Combine main and diagonal gradients with weights
        let dx = dx_main * 0.7 + dx_diag * 0.3;
        let dy = dy_main * 0.7 + dy_diag * 0.3;

        let gradient = Vec2::new(dx, dy);

        // Normalize the gradient to get the surface normal
        if gradient.length() > 0.001 {
            gradient.normalize()
        } else {
            // Fallback normal calculation if gradient is too small
            Vec2::Y
        }
    }

    /// Sample the Signed Distance Field at a specific world position
    /// Uses smoothed sampling for better continuity
    fn sample_sdf(world: &World, chunk_x: i32, chunk_y: i32, world_x: f32, world_y: f32) -> f32 {
        // Convert world position to voxel coordinates
        let voxel_x = world_x.floor() as i32;
        let voxel_y = world_y.floor() as i32;

        // Get the fractional part for interpolation
        let fx = world_x - voxel_x as f32;
        let fy = world_y - voxel_y as f32;

        // Apply smoothstep for smoother interpolation curves
        let smooth_fx = fx * fx * (3.0 - 2.0 * fx); // smoothstep
        let smooth_fy = fy * fy * (3.0 - 2.0 * fy); // smoothstep

        // Sample a 3x3 grid around the point for better filtering
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for dy in -1..=1 {
            for dx in -1..=1 {
                let sample_x = voxel_x + dx;
                let sample_y = voxel_y + dy;

                let voxel_value = Self::try_get_voxel(world, chunk_x, chunk_y, sample_x, sample_y)
                    .map(|solid| if solid { -1.0 } else { 1.0 })
                    .unwrap_or(0.0);

                // Calculate distance-based weight
                let dist_x = (dx as f32 - fx).abs();
                let dist_y = (dy as f32 - fy).abs();
                let distance = (dist_x * dist_x + dist_y * dist_y).sqrt();

                // Use gaussian-like weighting for smooth falloff
                let weight = (-distance * distance * 2.0).exp();

                weighted_sum += voxel_value * weight;
                total_weight += weight;
            }
        }

        if total_weight > 0.001 {
            weighted_sum / total_weight
        } else {
            // Fallback to simple bilinear interpolation
            let v00 = Self::try_get_voxel(world, chunk_x, chunk_y, voxel_x, voxel_y)
                .map(|solid| if solid { -1.0 } else { 1.0 })
                .unwrap_or(0.0);
            let v10 = Self::try_get_voxel(world, chunk_x, chunk_y, voxel_x + 1, voxel_y)
                .map(|solid| if solid { -1.0 } else { 1.0 })
                .unwrap_or(0.0);
            let v01 = Self::try_get_voxel(world, chunk_x, chunk_y, voxel_x, voxel_y + 1)
                .map(|solid| if solid { -1.0 } else { 1.0 })
                .unwrap_or(0.0);
            let v11 = Self::try_get_voxel(world, chunk_x, chunk_y, voxel_x + 1, voxel_y + 1)
                .map(|solid| if solid { -1.0 } else { 1.0 })
                .unwrap_or(0.0);

            // Use smoothed interpolation
            let v0 = v00 * (1.0 - smooth_fx) + v10 * smooth_fx;
            let v1 = v01 * (1.0 - smooth_fx) + v11 * smooth_fx;
            v0 * (1.0 - smooth_fy) + v1 * smooth_fy
        }
    }

    /// Solve Quadratic Error Function to find optimal vertex placement
    /// Enhanced version with better constraint handling and smoothing
    fn solve_qef(
        intersections: &[EdgeIntersection],
        _cell_x: usize,
        _cell_y: usize,
    ) -> (Vec2, Vec2) {
        if intersections.is_empty() {
            let center = Vec2::new(0.5, 0.5); // Center in local coordinates
            return (center, Vec2::Y);
        }

        // For better vertex placement, we'll solve a weighted least squares problem
        // The goal is to find a point that minimizes the distance to all the planes
        // defined by the intersection points and their normals

        let mut vertex_pos = Vec2::ZERO;
        let mut total_weight = 0.0;

        // First pass: weighted average of intersection positions
        for intersection in intersections {
            let weight = 1.0; // Equal weights for now, could be based on edge length or importance
            vertex_pos += intersection.position * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            vertex_pos /= total_weight;
        } else {
            vertex_pos = Vec2::new(0.5, 0.5);
        }

        // Second pass: iterative refinement using constraint projection
        for _iteration in 0..3 {
            let mut correction = Vec2::ZERO;
            let mut correction_weight = 0.0;

            for intersection in intersections {
                // Calculate how far the current vertex is from the constraint plane
                let to_intersection = intersection.position - vertex_pos;
                let distance_along_normal = to_intersection.dot(intersection.normal);

                // Project correction along the normal
                let constraint_correction = intersection.normal * distance_along_normal * 0.3; // Damping factor

                correction += constraint_correction;
                correction_weight += 1.0;
            }

            if correction_weight > 0.0 {
                vertex_pos += correction / correction_weight;
            }
        }

        // Calculate final normal as weighted average
        let mut avg_normal = Vec2::ZERO;
        let mut normal_weight = 0.0;

        for intersection in intersections {
            // Weight normals by how close they are to the final vertex position
            let distance = (intersection.position - vertex_pos).length();
            let weight = 1.0 / (1.0 + distance * 2.0); // Closer intersections have more influence

            avg_normal += intersection.normal * weight;
            normal_weight += weight;
        }

        let final_normal = if normal_weight > 0.001 && avg_normal.length() > 0.001 {
            avg_normal.normalize()
        } else {
            Vec2::Y
        };

        // Constrain the vertex to stay within the cell bounds with smooth clamping
        let margin = 0.1; // Small margin from edges for stability
        let constrained_pos = Vec2::new(
            vertex_pos.x.clamp(margin, 1.0 - margin),
            vertex_pos.y.clamp(margin, 1.0 - margin),
        );

        // Apply smoothstep near boundaries for smoother transitions
        let final_pos = Vec2::new(
            if constrained_pos.x < 0.2 || constrained_pos.x > 0.8 {
                // Apply smoothing near boundaries
                let t = if constrained_pos.x < 0.5 {
                    constrained_pos.x * 5.0
                } else {
                    (1.0 - constrained_pos.x) * 5.0
                };
                let smooth_t = t * t * (3.0 - 2.0 * t);
                if constrained_pos.x < 0.5 {
                    smooth_t * 0.2 + margin
                } else {
                    1.0 - (smooth_t * 0.2 + margin)
                }
            } else {
                constrained_pos.x
            },
            if constrained_pos.y < 0.2 || constrained_pos.y > 0.8 {
                let t = if constrained_pos.y < 0.5 {
                    constrained_pos.y * 5.0
                } else {
                    (1.0 - constrained_pos.y) * 5.0
                };
                let smooth_t = t * t * (3.0 - 2.0 * t);
                if constrained_pos.y < 0.5 {
                    smooth_t * 0.2 + margin
                } else {
                    1.0 - (smooth_t * 0.2 + margin)
                }
            } else {
                constrained_pos.y
            },
        );

        (final_pos, final_normal)
    }
}
