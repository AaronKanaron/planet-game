#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct DirtMaterial {
    color: vec4<f32>,
    mesh_size: vec3<f32>,
}

@group(2) @binding(0) var<uniform> material: DirtMaterial;

// Hash-funktion för randomisering
fn hash(p: vec2<f32>) -> vec2<f32> {
    let k = vec2<f32>(127.1, 311.7);
    let h = fract(sin(vec2<f32>(dot(p, k), dot(p, k.yx))) * 43758.5453);
    return h;
}

// Voronoi: returnerar avstånd till närmsta cell och ett cell-ID
fn voronoi(p: vec2<f32>) -> vec2<f32> {
    let i = floor(p);
    let f = fract(p);

    var min_dist = 8.0;
    var cell_id = vec2<f32>(0.0);

    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let neighbor = vec2<f32>(f32(x), f32(y));
            let point = hash(i + neighbor);
            let diff = neighbor + point - f;
            let dist = dot(diff, diff);
            if dist < min_dist {
                min_dist = dist;
                cell_id = point;
            }
        }
    }

    return vec2<f32>(sqrt(min_dist), cell_id.x); // avstånd och ID
}

// Enkel FBM för variation inom celler
fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var amp = 0.5;
    var freq = 1.0;
    for (var i = 0; i < 4; i = i + 1) {
        v += amp * (fract(sin(dot(p * freq, vec2<f32>(12.9898,78.233))) * 43758.5453) * 2.0 - 1.0);
        freq *= 2.0;
        amp *= 0.5;
    }
    return 0.5 + 0.5 * v;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv;
    let world_pos = mesh.world_position.xy;

    // Normalisera koordinater
    let coord = (uv + world_pos * 0.05) * material.mesh_size.xy / max(material.mesh_size.x, material.mesh_size.y);

    // Skala upp för mer variation
    let p = coord * 2.0;

    // Voronoi-baserad chunkning
    let v = voronoi(p);

    // Små variationer inom klumpar
    let grain = fbm(p * 1.3 + v.yx * 3.1);

    // Skapa färg (smutsfärger)
    let dark_dirt = vec3<f32>(0.22, 0.18, 0.12);
    let light_dirt = vec3<f32>(0.4, 0.3, 0.2);

    let dirt_color = mix(dark_dirt, light_dirt, grain);
    let chunk_shadow = 1.0 - smoothstep(0.0, 0.5, v.x); // skuggning vid kant av chunk

    let final_color = dirt_color * (0.7 + 0.3 * chunk_shadow); // förstärk skuggade kanter

    return vec4<f32>(final_color, 1.0);
}
