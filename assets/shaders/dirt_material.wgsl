#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct DirtMaterial {
    color: vec4<f32>,
    mesh_size: vec3<f32>,
}

@group(2) @binding(0) var<uniform> material: DirtMaterial;

// Hash function for pseudo-random numbers
fn hash2(p: vec2<f32>) -> vec2<f32> {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let p3_offset = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3_offset.xx + p3_offset.yz) * p3_offset.zy);
}

fn hash1(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let p3_offset = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3_offset.x + p3_offset.y) * p3_offset.z);
}

// Voronoi function that returns distance to nearest point and the point's hash
fn voronoi(uv: vec2<f32>, scale: f32) -> vec3<f32> {
    let scaled_uv = uv * scale;
    let i = floor(scaled_uv);
    let f = fract(scaled_uv);
    
    var min_dist = 1.0;
    var min_point = vec2<f32>(0.0);
    
    // Check 3x3 grid of neighboring cells
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let neighbor = vec2<f32>(f32(x), f32(y));
            let point = hash2(i + neighbor);
            let diff = neighbor + point - f;
            let dist = length(diff);
            
            if (dist < min_dist) {
                min_dist = dist;
                min_point = i + neighbor;
            }
        }
    }
    
    return vec3<f32>(min_dist, min_point);
}

// Fractal noise function
fn fractal_noise(uv: vec2<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var uv_offset = uv;
    
    for (var i = 0; i < octaves; i++) {
        value += amplitude * (hash1(floor(uv_offset * frequency)) * 2.0 - 1.0);
        amplitude *= 0.5;
        frequency *= 2.0;
        uv_offset = uv_offset * 1.7 + vec2<f32>(0.35, 0.65); // Rotate and offset for variation
    }
    
    return value * 0.5 + 0.5;
}

// Smooth minimum function for blending
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    let uv = mesh.uv;
    let world_pos = mesh.world_position.xy;
    
    // Use world position for non-repeating patterns
    let coords = world_pos * 0.01 + uv * 2.0;
    
    // Multiple scales of Voronoi for different dirt chunk sizes
    let voronoi1 = voronoi(coords, 8.0);  // Large chunks
    let voronoi2 = voronoi(coords, 16.0); // Medium chunks
    let voronoi3 = voronoi(coords, 32.0); // Small chunks
    
    // Get random values for each Voronoi cell
    let cell_hash1 = hash1(voronoi1.yz);
    let cell_hash2 = hash1(voronoi2.yz);
    let cell_hash3 = hash1(voronoi3.yz);
    
    // Create base dirt color variation using cell hashes
    let base_variation = cell_hash1 * 0.4 + cell_hash2 * 0.3 + cell_hash3 * 0.3;
    
    // Distance-based shading for chunk definition
    let edge_factor1 = smoothstep(0.0, 0.15, voronoi1.x);
    let edge_factor2 = smoothstep(0.0, 0.08, voronoi2.x);
    let edge_factor3 = smoothstep(0.0, 0.04, voronoi3.x);
    
    // Combine edge factors for depth
    let depth = smin(smin(edge_factor1, edge_factor2, 0.1), edge_factor3, 0.05);
    
    // Add fractal noise for surface texture
    let surface_noise = fractal_noise(coords * 2.0, 4);
    let fine_noise = fractal_noise(coords * 8.0, 3) * 0.3;
    
    // Create dirt color palette
    let dirt_dark = vec3<f32>(0.25, 0.15, 0.08);   // Dark soil
    let dirt_medium = vec3<f32>(0.45, 0.32, 0.18); // Medium soil
    let dirt_light = vec3<f32>(0.65, 0.52, 0.35);  // Light soil/clay
    let dirt_organic = vec3<f32>(0.15, 0.12, 0.06); // Organic matter
    
    // Mix colors based on cell hashes and depth
    var final_color = dirt_medium;
    
    // Vary color based on cell characteristics
    if (cell_hash1 > 0.7) {
        final_color = mix(final_color, dirt_light, 0.6);
    } else if (cell_hash1 < 0.3) {
        final_color = mix(final_color, dirt_dark, 0.8);
    }
    
    if (cell_hash2 > 0.8) {
        final_color = mix(final_color, dirt_organic, 0.4);
    }
    
    // Apply depth shading
    final_color = mix(final_color * 0.6, final_color, depth);
    
    // Add surface texture variation
    final_color = final_color * (0.8 + surface_noise * 0.4 + fine_noise);
    
    // Add some moisture variation
    let moisture = fractal_noise(coords * 0.5 + vec2<f32>(100.0, 50.0), 3);
    if (moisture > 0.6) {
        final_color = final_color * 0.7; // Darker for wet areas
    }
    
    // Apply material color tint
    final_color = final_color * material.color.rgb;
    
    return vec4<f32>(final_color, material.color.a);
}