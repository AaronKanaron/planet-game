// terrain_compute.wgsl
@group(0) @binding(0)
var<storage, read_write> voxel_data: array<u32>;

@group(0) @binding(1)
var<uniform> params: TerrainParams;

struct TerrainParams {
    world_width: u32,
    world_height: u32,
    center_x: f32,
    center_y: f32,
    radius: f32,
    noise_scale: f32,
}

// Simple noise function (you might want to use a more sophisticated one)
fn simple_noise(x: f32, y: f32) -> f32 {
    let n = sin(x * 12.9898 + y * 78.233) * 43758.5453;
    return fract(n);
}

// Perlin-like noise approximation
fn perlin_noise(x: f32, y: f32) -> f32 {
    let i = floor(vec2<f32>(x, y));
    let f = fract(vec2<f32>(x, y));
    
    let a = simple_noise(i.x, i.y);
    let b = simple_noise(i.x + 1.0, i.y);
    let c = simple_noise(i.x, i.y + 1.0);
    let d = simple_noise(i.x + 1.0, i.y + 1.0);
    
    let u = f * f * (3.0 - 2.0 * f);
    
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y) * 2.0 - 1.0;
}

@compute @workgroup_size(8, 8, 1)
fn generate_terrain(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    // Check bounds
    if (x >= params.world_width || y >= params.world_height) {
        return;
    }
    
    let index = y * params.world_width + x;
    
    // Calculate distance from center
    let dx = f32(x) - params.center_x;
    let dy = f32(y) - params.center_y;
    let distance = sqrt(dx * dx + dy * dy);
    
    if (distance < params.radius) {
        // Generate noise
        let noise_value = perlin_noise(f32(x) / params.noise_scale, f32(y) / params.noise_scale);
        
        if (distance < params.radius - 5.0) {
            // Inner core - always rock
            voxel_data[index] = 1u; // Rock
        } else if (noise_value > 0.0) {
            // Outer ring with noise - dirt where noise is positive
            voxel_data[index] = 2u; // Dirt
        } else {
            // Air
            voxel_data[index] = 0u; // Air
        }
    } else {
        // Outside radius - air
        voxel_data[index] = 0u; // Air
    }
}