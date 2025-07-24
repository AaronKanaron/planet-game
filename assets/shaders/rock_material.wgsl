#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct RockMaterial {
    color: vec4<f32>,
    mesh_size: vec3<f32>,
}

@group(2) @binding(0) var<uniform> material: RockMaterial;

// Hash för pseudo-random värde
fn hash(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let p4 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p4.x + p4.y) * p4.z);
}

// Value noise
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));

    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Fractal Brownian Motion
fn fbm(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var i = 0; i < 5; i = i + 1) {
        value += amplitude * noise(p * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// Distortion function (turbulence-style)
fn distort(p: vec2<f32>) -> vec2<f32> {
    let q = vec2<f32>(
        fbm(p + vec2<f32>(0.0, 0.0)),
        fbm(p + vec2<f32>(5.2, 1.3))
    );
    return p + 0.5 * q;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv;
    let world_pos = mesh.world_position.xy;

    // Kombinera UV med world-pos för variation
    let base_coords = (uv + world_pos * 0.05) * material.mesh_size.xy / max(material.mesh_size.x, material.mesh_size.y);

    // Distortion för att skapa organiska mönster
    let distorted = distort(base_coords * 2.0);

    // Färgvärde via fbm
    let n = fbm(distorted * 1.5);

    // Tona mellan två stenspecifika färger
    let rock_dark = vec3<f32>(0.15, 0.13, 0.12);
    let rock_light = vec3<f32>(0.6, 0.55, 0.5);
    let color = mix(rock_dark, rock_light, n);

    return vec4<f32>(color, 1.0);
}
