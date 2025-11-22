use physics_sim::shader_gen::{SdfOp, TransformOp}; // Import the Tree DNA
use physics_sim::gpu_data::{GpuMagneticParticle, GeneticSimParams}; 
use wgpu::util::DeviceExt;
use rand::Rng;
use rand::seq::SliceRandom;

// Genetic Algorithm Parameters
const POPULATION_SIZE: usize = 20; // Smaller population because compilation is heavy
const PARTICLES_PER_CHAMBER: usize = 1000;
const GENERATIONS: usize = 10;
const MUTATION_RATE: f32 = 0.3; // Higher mutation for tree structure

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

// Helper functions
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
"#;

const SHADER_FOOTER: &str = r#" 
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
"#;

#[tokio::main]
async fn main() {
    env_logger::init();
    println!("Initializing True Genetic Programming Engine...");

    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::Performance,
        ..Default::default()
    }).await.unwrap();

    // Initial Population of Trees
    let mut population: Vec<SdfOp> = (0..POPULATION_SIZE).map(|_| SdfOp::random(4)).collect();
    let mut fitness_scores: Vec<f32> = vec![0.0; POPULATION_SIZE];
    let mut best_code = String::new();
    let mut best_fitness = -1.0;

    let mut rng = rand::rng();

    for generation_num in 0..GENERATIONS {
        println!("\n--- Generation {} ---", generation_num);

        for (idx, genome) in population.iter().enumerate() {
            // 1. Generate Shader Source
            let geometry_body = format!("    return {};", genome.to_wgsl("p"));
            let full_shader = format!("{}{}{}", SHADER_HEADER, geometry_body, SHADER_FOOTER);

            // 2. Compile Shader
            // We wrap this in a try-block (conceptually) because random trees might produce invalid math (rare but possible)
            let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("Gen {} Ind {}", generation_num, idx)),
                source: wgpu::ShaderSource::Wgsl(full_shader.into()),
            });

            // 3. Setup Simulation
            let particles: Vec<GpuMagneticParticle> = (0..PARTICLES_PER_CHAMBER).map(|_| {
                GpuMagneticParticle::new(
                    rng.random_range(0.0..CHAMBER_DIMS[0]),
                    rng.random_range(0.0..CHAMBER_DIMS[1]),
                    rng.random_range(0.0..CHAMBER_DIMS[2]),
                    0.0
                )
            }).collect();

            let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particle Buffer"),
                contents: bytemuck::cast_slice(&particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            });

            let sim_params = GeneticSimParams {
                dt: SIM_DT,
                temp_gradient_start_x: TEMP_GRADIENT_START_X,
                temp_gradient_end_x: TEMP_GRADIENT_END_X,
                max_temp_noise_mag: MAX_THERMAL_NOISE,
                chamber_dims: CHAMBER_DIMS,
                _pad_chamber_dims: 0.0,
                genome_genes: [0.0; 16], // Unused in GP mode
            };
            let param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Param Buffer"),
                contents: bytemuck::cast_slice(&[sim_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            // Pipeline
            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &
                    [
                        wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                        wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                    ],
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None, layout: &bind_group_layout,
                entries: &
                    [
                        wgpu::BindGroupEntry { binding: 0, resource: particle_buffer.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: param_buffer.as_entire_binding() },
                    ],
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&bind_group_layout], push_constant_ranges: &[] });
            
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None, layout: Some(&pipeline_layout), module: &shader_module, entry_point: Some("main"), compilation_options: Default::default(), cache: None,
            });

            // Run
            for _ in 0..SIM_STEPS {
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
                    cpass.set_pipeline(&pipeline);
                    cpass.set_bind_group(0, &bind_group, &[]);
                    let wgs = (PARTICLES_PER_CHAMBER as u32 + 64 - 1) / 64;
                    cpass.dispatch_workgroups(wgs, 1, 1);
                }
                queue.submit(Some(encoder.finish()));
            }
            
            // Readback
            let buffer_size = (PARTICLES_PER_CHAMBER * std::mem::size_of::<GpuMagneticParticle>()) as wgpu::BufferAddress;
            let staging = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: buffer_size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            encoder.copy_buffer_to_buffer(&particle_buffer, 0, &staging, 0, buffer_size);
            queue.submit(Some(encoder.finish()));
            
            let (tx, rx) = tokio::sync::oneshot::channel();
            let slice = staging.slice(..);
            slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
            device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();
            rx.await.unwrap().unwrap();
            
            let data = slice.get_mapped_range();
            let results: &[GpuMagneticParticle] = bytemuck::cast_slice(&data);
            
            let total_flow: f32 = results.iter().map(|p| p.properties[0]).sum();
            // Sanitize NaN/Inf
            fitness_scores[idx] = if total_flow.is_nan() || total_flow.is_infinite() {
                -1.0 // Penalize broken physics
            } else {
                total_flow
            };
            
            // Cleanup (Drop mapped range)
            drop(data);
            staging.unmap();
        }

        // --- Breeding ---
        let mut sorted_indices: Vec<usize> = (0..POPULATION_SIZE).collect();
        sorted_indices.sort_by(|&a, &b| fitness_scores[b].partial_cmp(&fitness_scores[a]).unwrap_or(std::cmp::Ordering::Equal));
        
        println!("  Best Flow: {:.4}", fitness_scores[sorted_indices[0]]);
        
        if fitness_scores[sorted_indices[0]] > best_fitness {
            best_fitness = fitness_scores[sorted_indices[0]];
            best_code = population[sorted_indices[0]].to_wgsl("p");
        }

        if generation_num < GENERATIONS - 1 {
            let mut next_pop = Vec::new();
            let elite_count = POPULATION_SIZE / 4;
            
            for i in 0..POPULATION_SIZE {
                if i < elite_count {
                    next_pop.push(population[sorted_indices[i]].clone());
                } else {
                    // Mate two random elites
                    let p1 = &population[sorted_indices[rng.random_range(0..elite_count)]];
                    let p2 = &population[sorted_indices[rng.random_range(0..elite_count)]];
                    let mut child = SdfOp::crossover(p1, p2);
                    child = child.mutate(MUTATION_RATE);
                    next_pop.push(child);
                }
            }
            population = next_pop;
        }
    }
    
    println!("\n--- Evolution Complete ---");
    println!("Best WGSL Code found:\n{}", best_code);
}
