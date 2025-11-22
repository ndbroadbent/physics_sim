// Physics Simulation Shader

struct Particle {
    position: vec4<f32>, // x, y, z, mass
    velocity: vec4<f32>, // vx, vy, vz, charge
    properties: vec4<f32>, // energy, type, active, padding
};

struct SimParams {
    width: f32,
    height: f32,
    atom_density: f32,
    band_gap: f32,
    dt: f32,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: SimParams;

// Simple pseudo-random generator
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

    // 1. Check if active
    if (p.properties.z < 0.5) {
        return;
    }

    // 2. Move Particle
    // p.position.x += p.velocity.x * params.dt;
    // p.position.y += p.velocity.y * params.dt;
    // WGSL vec arithmetic
    p.position = p.position + p.velocity * params.dt;

    // 3. Boundary Check (Simple Wrap/Kill)
    if (p.position.x > params.width) {
        p.properties.z = 0.0; // Kill
    }

    // 4. Interaction Logic (Simplified)
    // Check if we are in the material slab (x > 200 && x < 600)
    if (p.position.x > 200.0 && p.position.x < 600.0) {
        // Interaction probability
        let rng = rand(p.position.xy);
        
        if (p.properties.y == 0.0) { // Photon
             // Probability ~ density * dt
             if (rng < params.atom_density * 0.1) {
                 // Absorbed?
                 if (p.properties.x > params.band_gap) {
                     p.properties.z = 0.0; // Kill photon
                     // In a full sim, we would spawn electrons here. 
                     // But compute shaders can't easily "push" to the array without atomics.
                     // For now, just kill it (Photoelectric absorption).
                 }
             }
        }
    }

    // Write back
    particles[index] = p;
}
