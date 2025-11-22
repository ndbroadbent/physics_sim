use physics_sim::shader_gen::{SdfOp, TransformOp}; // Import the Tree DNA
use physics_sim::gpu_data::{GpuMagneticParticle, GeneticSimParams}; 
use wgpu::util::DeviceExt;
use rand::Rng;
use rand::seq::SliceRandom;
use std::fs;
use std::path::Path; // To save genomes

// Genetic Algorithm Parameters
const POPULATION_SIZE: usize = 50; // Scaled up
const PARTICLES_PER_CHAMBER: usize = 1000;
const GENERATIONS: usize = 20; // Longer run
const MUTATION_RATE: f32 = 0.4; // High mutation
const INITIAL_TREE_DEPTH: u32 = 5; // Deeper initial trees

// Simulation Parameters
const CHAMBER_DIMS: [f32; 3] = [30.0, 30.0, 30.0];
const SIM_DT: f32 = 0.1;
const SIM_STEPS: u32 = 200;
const MAX_THERMAL_NOISE: f32 = 0.5;
const TEMP_GRADIENT_START_X: f32 = 5.0;
const TEMP_GRADIENT_END_X: f32 = 25.0;

// Template shader parts
const SHADER_HEADER: &str = r#"
// 3D Genetic Simulation Shader (Generated)

// Helper functions (copied from shader_gen for convenience)
fn rotY(p: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(c * p.x + s * p.z, p.y, -s * p.x + c * p.z);
}

fn sdBox(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sdSphere(p: vec3<f32>, s: f32) -> f32 {
    return length(p) - s;
}

// Particle Structs
struct MagneticParticle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    magnetic_moment: vec4<f32>,
    properties: vec4<f32>,
};

struct GeneticSimParams {
    dt: f32,
    temp_gradient_start_x: f32,
    temp_gradient_end_x: f32,
    max_temp_noise_mag: f32,
    chamber_dims: vec3<f32>,
    _pad_chamber_dims: f32,
    genome_genes: array<vec4<f32>, 4>, // Unused in GP mode, kept for struct alignment compatibility
};

@group(0) @binding(0) var<storage, read_write> particles: array<MagneticParticle>;
@group(0) @binding(1) var<uniform> sim_params: GeneticSimParams;

// Random
fn rand(seed: vec2<f32>) -> f32 {
    return fract(sin(dot(seed, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// --- GENERATED GEOMETRY FUNCTION ---
fn map_geometry(p: vec3<f32>) -> f32 {
// INSERT_GENERATED_CODE_HERE
}

// Normal calculation using the generated map_geometry
fn get_normal(p: vec3<f32>) -> vec3<f32> {
    let eps = 0.001;
    let normal_x = map_geometry(p + vec3<f32>(eps, 0.0, 0.0)) - map_geometry(p - vec3<f32>(eps, 0.0, 0.0));
    let normal_y = map_geometry(p + vec3<f32>(0.0, eps, 0.0)) - map_geometry(p - vec3<f32>(0.0, eps, 0.0));
    let normal_z = map_geometry(p + vec3<f32>(0.0, 0.0, eps)) - map_geometry(p - vec3<f32>(0.0, 0.0, eps));
    return normalize(vec3<f32>(normal_x, normal_y, normal_z));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&particles)) { return; }

    var p = particles[index];
    if (p.properties.y < 0.5) { return; }

    // Thermal Noise
    let temp_norm = 1.0 - (p.position.x - sim_params.temp_gradient_start_x) / (sim_params.temp_gradient_end_x - sim_params.temp_gradient_start_x);
    let temp_factor = clamp(temp_norm, 0.0, 1.0);
    let noise_mag = sim_params.max_temp_noise_mag * sqrt(temp_factor);
    let seed_x = p.position.x * 0.1 + sim_params.dt * f32(global_id.x) * 0.001;
    let seed_y = p.position.y * 0.1 + sim_params.dt * f32(global_id.y) * 0.001;
    let seed_z = p.position.z * 0.1 + sim_params.dt * f32(global_id.z) * 0.001;
    p.velocity.x += (rand(vec2<f32>(seed_x, seed_y)) * 2.0 - 1.0) * noise_mag;
    p.velocity.y += (rand(vec2<f32>(seed_y, seed_z)) * 2.0 - 1.0) * noise_mag;
    p.velocity.z += (rand(vec2<f32>(seed_z, seed_x)) * 2.0 - 1.0) * noise_mag;

    // Move
    p.position = p.position + vec4<f32>(p.velocity.xyz * sim_params.dt, 0.0);

    // Collision
    let dist = map_geometry(p.position.xyz);
    if (dist < 0.0) {
        let n = get_normal(p.position.xyz);
        p.velocity = vec4<f32>(reflect(p.velocity.xyz, n), 0.0);
        p.position = p.position + vec4<f32>(n * (-dist + 0.001), 0.0);
    }

    // Boundaries & Recycle
    if (p.position.x < 0.0) {
        p.position.x = sim_params.temp_gradient_start_x;
        p.velocity.x = 0.0;
    } else if (p.position.x > sim_params.chamber_dims.x) {
        p.properties.x += sim_params.chamber_dims.x; // Score!
        p.position.x = sim_params.temp_gradient_start_x;
        p.velocity = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    if (p.position.y < 0.0 || p.position.y > sim_params.chamber_dims.y) {
        p.velocity.y *= -1.0; p.position.y = clamp(p.position.y, 0.0, sim_params.chamber_dims.y);
    }
    if (p.position.z < 0.0 || p.position.z > sim_params.chamber_dims.z) {
        p.velocity.z *= -1.0; p.position.z = clamp(p.position.z, 0.0, sim_params.chamber_dims.z);
    }

    p.properties.x += p.velocity.x * sim_params.dt;
    particles[index] = p;
}