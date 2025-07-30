/* Imports */
use crate::planet::{
    meshing::render_voxels::VOXEL_SIZE,
    world::{chunk::CHUNK_SIZE, chunk_world::World},
};
use bevy::{platform::collections::HashMap, prelude::*};

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

/// Snapped coordinate for consistent vertex positioning across chunks
/// Uses fixed-point integer coordinates (multiplied by 1000) for HashMap keys
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

    pub fn to_world_pos(&self) -> Vec2 {
        Vec2::new(self.x as f32 / 1000.0, self.y as f32 / 1000.0)
    }
}

/// Represents a vertex on a chunk border that can be shared
#[derive(Debug, Clone)]
pub struct BorderVertex {
    pub world_position: Vec2,
    pub normal: Vec2,
    pub vertex_id: usize,
}

/// Global registry for shared border vertices
#[derive(Debug, Default)]
pub struct SharedVertexRegistry {
    vertices: HashMap<SnappedCoord, BorderVertex>,
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
    pub fn get_or_create_vertex(
        &mut self,
        coord: SnappedCoord,
        world_pos: Vec2,
        normal: Vec2,
    ) -> usize {
        if let Some(vertex) = self.vertices.get(&coord) {
            vertex.vertex_id
        } else {
            let vertex_id = self.next_vertex_id;
            self.next_vertex_id += 1;

            self.vertices.insert(
                coord,
                BorderVertex {
                    world_position: world_pos,
                    normal,
                    vertex_id,
                },
            );

            vertex_id
        }
    }

    /// Get a vertex
    pub fn get_vertex(&self, coord: &SnappedCoord) -> Option<&BorderVertex> {
        self.vertices.get(coord)
    }
}

/// Information about a border intersection
#[derive(Debug, Clone)]
pub struct BorderIntersection {
    pub world_position: Vec2,
    pub normal: Vec2,
    pub edge_type: BorderEdgeType,
    pub snapped_coord: SnappedCoord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderEdgeType {
    Left,   // x = 0
    Right,  // x = CHUNK_SIZE
    Bottom, // y = 0
    Top,    // y = CHUNK_SIZE
}

pub struct DualContouring;

/* Implementations */
impl DualContouring {
    /// Find contour cells and detect border intersections for shared vertex system
    pub fn find_contour_cells_with_borders(
        world: &World,
        chunk_x: i32,
        chunk_y: i32,
        vertex_registry: &mut SharedVertexRegistry,
    ) -> (Vec<ContourCell>, Vec<BorderIntersection>) {
        let mut contour_cells = Vec::new();
        let mut border_intersections = Vec::new();

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
                let corner_opts = [prev_row[x], prev_row[x + 1], curr_row[x], curr_row[x + 1]];

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
                    let (interior_vertex, normal) =
                        Self::solve_qef_for_cell(world, chunk_x, chunk_y, x, y, &corners);

                    let cell_border_intersections = Self::find_border_intersections(
                        world,
                        chunk_x,
                        chunk_y,
                        x,
                        y,
                        &corners,
                        vertex_registry,
                    );
                    border_intersections.extend(cell_border_intersections);

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

        (contour_cells, border_intersections)
    }

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
                let corner_opts = [prev_row[x], prev_row[x + 1], curr_row[x], curr_row[x + 1]];

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

        let edges = [(0, 1), (1, 3), (3, 2), (2, 0)];

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
        let local_pos = match edge_idx {
            0 => Vec2::new(0.5, 0.0),
            1 => Vec2::new(1.0, 0.5),
            2 => Vec2::new(0.5, 1.0),
            3 => Vec2::new(0.0, 0.5),
            _ => unreachable!(),
        };

        let world_pos = Vec2::new(cell_x as f32 + local_pos.x, cell_y as f32 + local_pos.y);

        let normal = Self::calculate_sdf_gradient(world, chunk_x, chunk_y, world_pos);

        EdgeIntersection {
            position: local_pos,
            normal,
        }
    }

    /// Calculate SDF gradient at a specific world position for proper normal computation
    fn calculate_sdf_gradient(world: &World, chunk_x: i32, chunk_y: i32, world_pos: Vec2) -> Vec2 {
        let epsilon = 0.05;

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

        let dx_main = (samples[1] - samples[0]) / (2.0 * epsilon);
        let dy_main = (samples[3] - samples[2]) / (2.0 * epsilon);

        let dx_diag = ((samples[5] + samples[7]) - (samples[4] + samples[6])) / (4.0 * epsilon);
        let dy_diag = ((samples[6] + samples[7]) - (samples[4] + samples[5])) / (4.0 * epsilon);

        let dx = dx_main * 0.7 + dx_diag * 0.3;
        let dy = dy_main * 0.7 + dy_diag * 0.3;

        let gradient = Vec2::new(dx, dy);

        if gradient.length() > 0.001 {
            gradient.normalize()
        } else {
            Vec2::Y
        }
    }

    /// Sample the Signed Distance Field at a specific world position
    /// Uses smoothed sampling for better continuity
    fn sample_sdf(world: &World, chunk_x: i32, chunk_y: i32, world_x: f32, world_y: f32) -> f32 {
        let voxel_x = world_x.floor() as i32;
        let voxel_y = world_y.floor() as i32;

        let fx = world_x - voxel_x as f32;
        let fy = world_y - voxel_y as f32;

        let smooth_fx = fx * fx * (3.0 - 2.0 * fx);
        let smooth_fy = fy * fy * (3.0 - 2.0 * fy);

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

            let v0 = v00 * (1.0 - smooth_fx) + v10 * smooth_fx;
            let v1 = v01 * (1.0 - smooth_fx) + v11 * smooth_fx;
            v0 * (1.0 - smooth_fy) + v1 * smooth_fy
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
    ) -> Vec<BorderIntersection> {
        let mut border_intersections = Vec::new();

        let edges = [(0, 1, 0), (1, 3, 1), (3, 2, 2), (2, 0, 3)];

        for (corner1_idx, corner2_idx, edge_idx) in edges.iter() {
            let corner1_solid = corners[*corner1_idx];
            let corner2_solid = corners[*corner2_idx];

            if corner1_solid != corner2_solid {
                if let Some(border_type) = Self::get_border_type(cell_x, cell_y, *edge_idx) {
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

                    let chunk_world_x = chunk_x as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE;
                    let chunk_world_y = chunk_y as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE;
                    let cell_world_x = chunk_world_x + cell_x as f32 * VOXEL_SIZE;
                    let cell_world_y = chunk_world_y + cell_y as f32 * VOXEL_SIZE;

                    let world_pos = Vec2::new(
                        cell_world_x + intersection.position.x * VOXEL_SIZE,
                        cell_world_y + intersection.position.y * VOXEL_SIZE,
                    );

                    let snapped_coord = SnappedCoord::from_world_pos(world_pos);

                    let _vertex_id = vertex_registry.get_or_create_vertex(
                        snapped_coord,
                        world_pos,
                        intersection.normal,
                    );

                    border_intersections.push(BorderIntersection {
                        world_position: world_pos,
                        normal: intersection.normal,
                        edge_type: border_type,
                        snapped_coord,
                    });
                }
            }
        }

        border_intersections
    }

    /// Determine if an edge crosses a chunk boundary and return the border type
    fn get_border_type(cell_x: usize, cell_y: usize, edge_idx: usize) -> Option<BorderEdgeType> {
        match edge_idx {
            0 => {
                if cell_y == 0 {
                    Some(BorderEdgeType::Bottom)
                } else {
                    None
                }
            }
            1 => {
                if cell_x == CHUNK_SIZE - 1 {
                    Some(BorderEdgeType::Right)
                } else {
                    None
                }
            }
            2 => {
                if cell_y == CHUNK_SIZE - 1 {
                    Some(BorderEdgeType::Top)
                } else {
                    None
                }
            }
            3 => {
                if cell_x == 0 {
                    Some(BorderEdgeType::Left)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Calculate exact intersection point using line-line intersection for border edges
    fn calculate_border_intersection(
        p1: Vec2,
        p2: Vec2,
        border_line_start: Vec2,
        border_line_end: Vec2,
    ) -> Option<Vec2> {
        let s1 = p2 - p1;
        let s2 = border_line_end - border_line_start;

        let denominator = s1.x * s2.y - s2.x * s1.y;

        if denominator.abs() < 0.0001 {
            return None;
        }

        let t = ((border_line_start.x - p1.x) * s2.y - (border_line_start.y - p1.y) * s2.x)
            / denominator;
        let u = ((border_line_start.x - p1.x) * s1.y - (border_line_start.y - p1.y) * s1.x)
            / denominator;

        if t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0 {
            Some(p1 + s1 * t)
        } else {
            None
        }
    }

    /// Solve Quadratic Error Function to find optimal vertex placement
    fn solve_qef(
        intersections: &[EdgeIntersection],
        _cell_x: usize,
        _cell_y: usize,
    ) -> (Vec2, Vec2) {
        if intersections.is_empty() {
            let center = Vec2::new(0.5, 0.5);
            return (center, Vec2::Y);
        }

        let mut vertex_pos = Vec2::ZERO;
        let mut total_weight = 0.0;

        for intersection in intersections {
            let weight = 1.0;
            vertex_pos += intersection.position * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            vertex_pos /= total_weight;
        } else {
            vertex_pos = Vec2::new(0.5, 0.5);
        }

        for _iteration in 0..3 {
            let mut correction = Vec2::ZERO;
            let mut correction_weight = 0.0;

            for intersection in intersections {
                let to_intersection = intersection.position - vertex_pos;
                let distance_along_normal = to_intersection.dot(intersection.normal);

                let constraint_correction = intersection.normal * distance_along_normal * 0.3; // Damping factor

                correction += constraint_correction;
                correction_weight += 1.0;
            }

            if correction_weight > 0.0 {
                vertex_pos += correction / correction_weight;
            }
        }

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

        let margin = 0.1;
        let constrained_pos = Vec2::new(
            vertex_pos.x.clamp(margin, 1.0 - margin),
            vertex_pos.y.clamp(margin, 1.0 - margin),
        );

        let final_pos = Vec2::new(
            if constrained_pos.x < 0.2 || constrained_pos.x > 0.8 {
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

    /// Helper method to get shared vertex information for a specific coordinate
    pub fn get_shared_vertex_info(
        world: &World,
        snapped_coord: SnappedCoord,
    ) -> Option<&BorderVertex> {
        world.get_vertex_registry().get_vertex(&snapped_coord)
    }

    /// Clear the shared vertex registry (useful when regenerating large areas)
    pub fn clear_vertex_registry(world: &mut World) {
        let registry = world.get_vertex_registry_mut();
        registry.vertices.clear();
        registry.next_vertex_id = 0;
    }
}
