struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    properties: vec4<f32>, // energy, type, active, layer_absorbed_index
};

struct SimParams {
    width: f32,
    height: f32,
    atom_density: f32,
    dt: f32,
    
    l1_start: f32, l1_end: f32, l1_band_gap: f32, l1_active: f32,
    l2_start: f32, l2_end: f32, l2_band_gap: f32, l2_active: f32,
    l3_start: f32, l3_end: f32, l3_band_gap: f32, l3_active: f32,
    
    padding: vec3<f32>,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: SimParams;

fn rand(seed: vec2<f32>) -> f32 {
    return fract(sin(dot(seed, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&particles)) {
        return;
    }

    var p = particles[index];

    if (p.properties.z < 0.5) {
        return;
    }

    p.position = p.position + p.velocity * params.dt;

    if (p.position.x > params.width) {
        p.properties.z = 0.0; 
    }

    // Multi-Layer Interaction Logic
    let x = p.position.x;
    let energy = p.properties.x;
    let rng = rand(p.position.xy);
    let prob = params.atom_density * params.dt; // Simplified

    if (p.properties.y == 0.0) { // Photon
        if (rng < prob) {
            // Layer 1 Check
            if (params.l1_active > 0.5 && x >= params.l1_start && x < params.l1_end) {
                if (energy >= params.l1_band_gap) {
                    p.properties.z = 0.0; // Absorbed
                    p.properties.w = 1.0; // Tag as absorbed by Layer 1
                }
            }
            // Layer 2 Check
            else if (params.l2_active > 0.5 && x >= params.l2_start && x < params.l2_end) {
                if (energy >= params.l2_band_gap) {
                    p.properties.z = 0.0; // Absorbed
                    p.properties.w = 2.0; // Tag as absorbed by Layer 2
                }
            }
            // Layer 3 Check
            else if (params.l3_active > 0.5 && x >= params.l3_start && x < params.l3_end) {
                if (energy >= params.l3_band_gap) {
                    p.properties.z = 0.0; // Absorbed
                    p.properties.w = 3.0; // Tag as absorbed by Layer 3
                }
            }
        }
    }

    particles[index] = p;
}