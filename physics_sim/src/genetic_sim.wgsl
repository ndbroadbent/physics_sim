// 3D Genetic Simulation Shader

// --- Start Inlined geometry.wgsl ---

// A Genome defines the parameters for constructing the geometry
struct Genome {
    params: array<vec4<f32>, 4>, // 16 genes, passed as 4 vec4s for alignment
};

// Helper to extract genes from the packed vec4 array
fn get_gene(index: i32, genes: array<vec4<f32>, 4>) -> f32 {
    let vec_idx = index / 4;
    let comp_idx = index % 4;
    let v = genes[vec_idx];
    if (comp_idx == 0) { return v.x; }
    if (comp_idx == 1) { return v.y; }
    if (comp_idx == 2) { return v.z; }
    return v.w;
}

// Rotates a point p by angle 'a' around Y axis
fn rotY(p: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(c * p.x + s * p.z, p.y, -s * p.x + c * p.z);
}

// --- Primitives ---

// Box
fn sdBox(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

// --- Fractal Generation ---

// A simple Menger Sponge-like iteration or similar IFS
fn sdFractalChannel(pos: vec3<f32>, genes_array: array<vec4<f32>, 4>) -> f32 {
    var p = pos;
    var d = sdBox(p, vec3<f32>(20.0, 20.0, 20.0)); // Bounding box. Reduced size for better detail.

    let iterations = 4;
    
    // Unpack genes using helper
    let scale = get_gene(0, genes_array) * 2.0 + 1.0; 
    let offset = vec3<f32>(get_gene(1, genes_array), get_gene(2, genes_array), get_gene(3, genes_array)) * 5.0; 
    let rot_angle = get_gene(4, genes_array) * 3.14159 * 2.0; 

    for (var i = 0; i < iterations; i++) {
        p = abs(p + offset) - offset; 
        p = rotY(p, rot_angle);       
        p = p * scale;                
        // Subtract box to create channels
        let box_dist = sdBox(p, vec3<f32>(1.0));
        let scale_factor = pow(scale, f32(i + 1));
        d = max(d, -box_dist / scale_factor);
    }
    
    return d; 
}


// --- New: Sawtooth Ratchet Primitive ---
// A simple 2D sawtooth extruded in Z
fn sdSawtoothRatchet(p_in: vec3<f32>, genes_array: array<vec4<f32>, 4>) -> f32 {
    var p = p_in;
    p.x = p.x % 5.0; // Repeat every 5 units in X
    
    let tooth_height = get_gene(0, genes_array) * 4.0 + 1.0; // Gene 0: height (1 to 5)
    let tooth_angle = get_gene(1, genes_array) * 0.5 + 0.1; // Gene 1: angle (small bias)
    let wall_thickness = 0.5;

    // A single V-shape.
    // Shift p to make the origin at the tip of the V
    p.x -= 2.5;
    p.y -= tooth_height; 

    // Rotate the space to align with one side of the V
    let a = atan2(p.y, p.x);
    let l = length(p.xy);
    
    // Define the V-shape using two planes
    // Angle 1
    let d1 = dot(p.xy, vec2<f32>(cos(tooth_angle), sin(tooth_angle)));
    // Angle 2 (negative of angle 1, to make the V)
    let d2 = dot(p.xy, vec2<f32>(cos(-tooth_angle), sin(-tooth_angle)));

    // Combined shape
    let d_v = max(d1, d2) - wall_thickness; // Extrude into a V
    
    // Subtract a flat base
    let base = p.y + tooth_height; // Distance to the floor
    
    return max(d_v, -base); // The shape is the V, constrained by the floor
}


// Main evaluation function
fn map_geometry(p: vec3<f32>, genes_array: array<vec4<f32>, 4>) -> f32 {
    // We will use the simple sawtooth ratchet for now
    // return sdSawtoothRatchet(p, genes_array);
    return sdFractalChannel(p, genes_array); // Switch back to fractal!
}

// Function to calculate surface normal
fn get_normal(p: vec3<f32>, genes_array: array<vec4<f32>, 4>) -> vec3<f32> {
    let eps = 0.001;
    let normal_x = map_geometry(p + vec3<f32>(eps, 0.0, 0.0), genes_array) - map_geometry(p - vec3<f32>(eps, 0.0, 0.0), genes_array);
    let normal_y = map_geometry(p + vec3<f32>(0.0, eps, 0.0), genes_array) - map_geometry(p - vec3<f32>(0.0, eps, 0.0), genes_array);
    let normal_z = map_geometry(p + vec3<f32>(0.0, 0.0, eps), genes_array) - map_geometry(p - vec3<f32>(0.0, 0.0, eps), genes_array);
    return normalize(vec3<f32>(normal_x, normal_y, normal_z));
}

// --- End Inlined geometry.wgsl ---


// Particle representing a magnetic dipole
struct MagneticParticle {
    position: vec4<f32>, // x, y, z, padding
    velocity: vec4<f32>, // vx, vy, vz, padding
    magnetic_moment: vec4<f32>, // mx, my, mz, padding (or just magnitude)
    properties: vec4<f32>, // current_flow_accum, active_flag, padding, padding
};

// Simulation parameters for the genetic algorithm
struct GeneticSimParams {
    dt: f32, // Time step
    temp_gradient_start_x: f32, // X-coord where temperature is highest
    temp_gradient_end_x: f32,   // X-coord where temperature is lowest
    max_temp_noise_mag: f32,    // Magnitude of thermal noise at highest temp
    chamber_dims: vec3<f32>,    // Dimensions of the 3D simulation chamber
    _pad_chamber_dims: f32, // Match Rust struct padding
    
    // Per-genome data (for the N-th chamber)
    genome_genes: array<vec4<f32>, 4>, // The DNA for this chamber's geometry
};

@group(0) @binding(0) var<storage, read_write> particles: array<MagneticParticle>;
@group(0) @binding(1) var<uniform> sim_params: GeneticSimParams;

// Simple pseudo-random generator (from before)
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

    if (p.properties.y < 0.5) { // Check if particle is active (properties.y is active_flag)
        return;
    }

    // 1. Apply Thermal Noise (Brownian Motion)
    let temp_norm = 1.0 - (p.position.x - sim_params.temp_gradient_start_x) / 
                      (sim_params.temp_gradient_end_x - sim_params.temp_gradient_start_x);
    let temp_factor = clamp(temp_norm, 0.0, 1.0); // 1.0 at hot end, 0.0 at cold end
    let noise_mag = sim_params.max_temp_noise_mag * sqrt(temp_factor);

    // Use position and time for random seed
    let seed_x = p.position.x * 0.1 + sim_params.dt * f32(global_id.x) * 0.001; // dt and global_id for variety
    let seed_y = p.position.y * 0.1 + sim_params.dt * f32(global_id.y) * 0.001;
    let seed_z = p.position.z * 0.1 + sim_params.dt * f32(global_id.z) * 0.001;

    let noise_vx = (rand(vec2<f32>(seed_x, seed_y)) * 2.0 - 1.0) * noise_mag;
    let noise_vy = (rand(vec2<f32>(seed_y, seed_z)) * 2.0 - 1.0) * noise_mag;
    let noise_vz = (rand(vec2<f32>(seed_z, seed_x)) * 2.0 - 1.0) * noise_mag;

    p.velocity.x += noise_vx;
    p.velocity.y += noise_vy;
    p.velocity.z += noise_vz;

    // 2. Move Particle
    p.position = p.position + vec4<f32>(p.velocity.xyz * sim_params.dt, 0.0);

    // 3. Collision with Evolved Geometry (Ratchet)
    // Pass the genome genes to the map_geometry and get_normal functions
    let dist_to_surface = map_geometry(p.position.xyz, sim_params.genome_genes); // Pass directly
    
    if (dist_to_surface < 0.0) { // Particle is inside the solid geometry (collision!)
        let normal = get_normal(p.position.xyz, sim_params.genome_genes); // Calculate surface normal
        
        // Simple Bounce: Reflect velocity
        p.velocity = vec4<f32>(reflect(p.velocity.xyz, normal), 0.0);
        
        // Push particle out slightly to avoid getting stuck
        p.position = p.position + vec4<f32>(normal * (-dist_to_surface + 0.001), 0.0); 
    }

    // 4. Boundary Conditions (Wrap around or kill)
    if (p.position.x < 0.0) { // Left (Hot) boundary
        p.position.x = sim_params.temp_gradient_start_x; // Push back to start of hot region
        p.velocity.x = 0.0; // Reset X velocity
    } else if (p.position.x > sim_params.chamber_dims.x) { // Right (Cold) boundary
        // Count flow and recycle to hot side
        p.properties.x += sim_params.chamber_dims.x; // Add full width as positive flow
        p.position.x = sim_params.temp_gradient_start_x; // Recycle to hot start
        p.velocity = vec4<f32>(0.0, 0.0, 0.0, 0.0); // Reset velocity
    }
    
    // Y and Z boundaries: simple bounce for now to keep them contained
    if (p.position.y < 0.0 || p.position.y > sim_params.chamber_dims.y) {
        p.velocity.y *= -1.0;
        p.position.y = clamp(p.position.y, 0.0, sim_params.chamber_dims.y);
    }
    if (p.position.z < 0.0 || p.position.z > sim_params.chamber_dims.z) {
        p.velocity.z *= -1.0;
        p.position.z = clamp(p.position.z, 0.0, sim_params.chamber_dims.z);
    }

    particles[index] = p;
}
