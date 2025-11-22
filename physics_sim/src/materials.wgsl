struct Atom {
    position: vec4<f32>, // x,y,z, mass
    velocity: vec4<f32>, // vx,vy,vz, type
    forces: vec4<f32>,   // fx,fy,fz, padding
};

struct SimParams {
    dt: f32,
    box_size: vec3<f32>,
    gravity: f32,
    pull_force: f32,
    padding: vec2<f32>,
};

@group(0) @binding(0) var<storage, read_write> atoms: array<Atom>;
@group(0) @binding(1) var<uniform> params: SimParams;

// Lennard-Jones Potential Force
// F = 24 * epsilon * (2*(sigma/r)^13 - (sigma/r)^7)
// Simplified for simulation stability: Spring-like near equilibrium
fn calculate_force(r: f32, r_eq: f32, strength: f32) -> f32 {
    if (r < 0.001) { return 0.0; } // Avoid div zero
    let strain = (r - r_eq); 
    // Hooke's law with cutoff? Or LJ?
    // Let's use a Morse potential approximation or simple spring for "bonds"
    // For now: Strong repulsion if r < r_eq, Attraction if r > r_eq (up to a cutoff)
    
    let k_stiff = 100.0 * strength;
    if (r > r_eq * 1.5) { return 0.0; } // Bond broken/Too far
    
    return -k_stiff * strain; // Negative force = attraction back to r_eq
}

@compute @workgroup_size(64)
fn integrate_forces(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&atoms)) { return; }

    var atom = atoms[index];
    var f = vec3<f32>(0.0, -params.gravity * atom.position.w, 0.0); // Gravity

    // Naive N^2 loop (Optimize with spatial grid later!)
    // For 10k atoms, N^2 is 100M interactions - too slow for 1 frame?
    // Metal compute can handle it, but grid is better.
    // For prototype, we limit N or use small neighborhood logic if implied by index?
    // Let's assume full N^2 for accuracy on small sample (1000-2000 atoms).
    
    let count = arrayLength(&atoms);
    for (var i = 0u; i < count; i++) {
        if (i == index) { continue; }
        let other = atoms[i];
        let delta = atom.position.xyz - other.position.xyz;
        let r = length(delta);
        
        // Interaction radius (cutoff)
        if (r < 4.0) {
            let dir = normalize(delta);
            // Ideal bond length (approx 2.5 Angstroms for metals)
            let r_eq = 2.5; 
            let force_mag = calculate_force(r, r_eq, 1.0);
            f += dir * force_mag;
        }
    }
    
    // Stress Test: Pull top layer
    if (atom.position.y > params.box_size.y * 0.9) {
        f.y += params.pull_force;
    }
    // Fix bottom layer
    if (atom.position.y < params.box_size.y * 0.1) {
        f = vec3<f32>(0.0);
        atom.velocity = vec4<f32>(0.0);
    }

    atoms[index].forces = vec4<f32>(f, 0.0);
}

@compute @workgroup_size(64)
fn integrate_motion(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&atoms)) { return; }

    var atom = atoms[index];
    
    // Verlet / Euler
    let a = atom.forces.xyz / atom.position.w;
    atom.velocity.x += a.x * params.dt;
    atom.velocity.y += a.y * params.dt;
    atom.velocity.z += a.z * params.dt;
    
    // Damping (Heat loss / friction)
    atom.velocity.x *= 0.99;
    atom.velocity.y *= 0.99;
    atom.velocity.z *= 0.99;

    atom.position.x += atom.velocity.x * params.dt;
    atom.position.y += atom.velocity.y * params.dt;
    atom.position.z += atom.velocity.z * params.dt;

    // Boundary (Floor)
    if (atom.position.y < 0.0) {
        atom.position.y = 0.0;
        atom.velocity.y = -atom.velocity.y * 0.5;
    }

    atoms[index] = atom;
}
