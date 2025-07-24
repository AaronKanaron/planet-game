#import bevy_sprite::mesh2d_vertex_output::VertexOutput
struct RockMaterial {
    color: vec4<f32>,
    mesh_size: vec3<f32>,
}
@group(2) @binding(0) var<uniform> material: RockMaterial;

// Hash function for random gradient
fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

// 2D value noise with smooth (Perlin-like) interpolation
fn noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Fractal Brownian Motion (adds layered noise at multiple scales)
fn fbm(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var i = 0u; i < 5u; i = i + 1u) {
        value = value + amplitude * noise2d(p * frequency);
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }
    return value;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv;
    let world_pos = mesh.world_position.xy;

    // Normalize to avoid stretching based on mesh size
    let normalized_uv = uv * material.mesh_size.xy / max(material.mesh_size.x, material.mesh_size.y);

    // Combine UV and world position for seamless, varied noise
    let pos = normalized_uv * 10.0 + world_pos * 0.01;

    // Fractal noise for the main rock pattern
    let n = fbm(pos);

    // Hard cracks/highlights based on noise thresholding
    let cracks = step(0.6, n);

    // Rock base color and variations
    let base_color = vec3<f32>(0.1, 0.1, 0.1);
    let color_variation = 0.5 * fbm(pos * 2.0);

    // Mix highlight (crack) color and add noise-based shading
    let final_color = mix(base_color, vec3<f32>(0.2, 0.1, 0.05), cracks) + color_variation;

    return vec4<f32>(final_color, 1.0);
}
