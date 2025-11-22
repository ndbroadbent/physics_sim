use physics_sim::genetic::Genome;
use physics_sim::gpu_data::{GpuMagneticParticle, GeneticSimParams};
use wgpu::util::DeviceExt;
// use std::time::Instant; // Not used yet
use rand::Rng; // For gen_range

// Genetic Algorithm Parameters
const POPULATION_SIZE: usize = 10;
const PARTICLES_PER_CHAMBER: usize = 1000; // Reduced for initial testing
const GENERATIONS: usize = 5; // How many generations to evolve
const MUTATION_RATE: f32 = 0.1;
const MUTATION_STRENGTH: f32 = 0.1;

// Simulation Parameters for each chamber
const CHAMBER_DIMS: [f32; 3] = [30.0, 30.0, 30.0]; // 3D dimensions of the nanostructure
const SIM_DT: f32 = 0.1;
const SIM_STEPS: u32 = 200; // Shorter simulation steps for faster iteration
const MAX_THERMAL_NOISE: f32 = 0.5; // Magnitude of thermal kicks
const TEMP_GRADIENT_START_X: f32 = 5.0; // Hot side
const TEMP_GRADIENT_END_X: f32 = 25.0; // Cold side

#[tokio::main]
async fn main() {
    env_logger::init();
    println!("Initializing 3D Genetic Physics Engine (Evolution Cycle)...");

    // 1. Setup GPU
    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::Performance,
        ..Default::default()
    }).await.unwrap();

    // 2. Initial Population
    let mut population: Vec<Genome> = (0..POPULATION_SIZE).map(|_| Genome::random()).collect();
    let mut fitness_scores: Vec<f32> = vec![0.0; POPULATION_SIZE];

    let mut best_genome: Genome = population[0].clone();
    let mut best_fitness: f32 = -1.0;

    println!("Generated initial population of {} genomes.", POPULATION_SIZE);

    // 3. Genetic Algorithm Loop
    for generation_num in 0..GENERATIONS { // Renamed 'gen' to 'generation_num'
        println!("\n--- Generation {} ---", generation_num);

        // --- Fitness Evaluation ---
        let total_particles = POPULATION_SIZE * PARTICLES_PER_CHAMBER;
        let mut initial_particles = Vec::with_capacity(total_particles);
        let mut rng = rand::thread_rng(); // Use rand::thread_rng()

        for i in 0..POPULATION_SIZE {
            for _ in 0..PARTICLES_PER_CHAMBER {
                // Spawn particles randomly in the chamber
                let x = rng.gen_range(0.0..CHAMBER_DIMS[0]);
                let y = rng.gen_range(0.0..CHAMBER_DIMS[1]);
                let z = rng.gen_range(0.0..CHAMBER_DIMS[2]);
                initial_particles.push(GpuMagneticParticle::new(x, y, z, i as f32)); // Store chamber ID
            }
        }

        let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Buffer"),
            contents: bytemuck::cast_slice(&initial_particles),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        });

        // Loop over each genome to run its simulation
        for (genome_idx, genome) in population.iter().enumerate() {
            // Update Uniforms for this specific genome's simulation
            let sim_params = GeneticSimParams {
                dt: SIM_DT,
                temp_gradient_start_x: TEMP_GRADIENT_START_X,
                temp_gradient_end_x: TEMP_GRADIENT_END_X,
                max_temp_noise_mag: MAX_THERMAL_NOISE,
                chamber_dims: CHAMBER_DIMS,
                genome_genes: genome.genes, // Pass genome genes
                _pad_chamber_dims: 0.0, // Match Rust struct padding
            };
            let sim_param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sim Param Buffer"),
                contents: bytemuck::cast_slice(&[sim_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            // Shader and Pipeline Setup (can optimize by creating once and reusing)
            let shader = device.create_shader_module(wgpu::include_wgsl!("genetic_sim.wgsl"));
            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None, }, count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1, visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None, }, count: None,
                    },
                ],
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None, layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: particle_buffer.as_entire_binding(), },
                    wgpu::BindGroupEntry { binding: 1, resource: sim_param_buffer.as_entire_binding(), },
                ],
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None, bind_group_layouts: &[&bind_group_layout], push_constant_ranges: &[],
            });
            let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None, layout: Some(&pipeline_layout), module: &shader, entry_point: Some("main"), compilation_options: Default::default(), cache: None,
            });

            // Dispatch Compute
            for _step in 0..SIM_STEPS {
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
                    cpass.set_pipeline(&compute_pipeline);
                    cpass.set_bind_group(0, &bind_group, &[]);
                    let workgroups = (PARTICLES_PER_CHAMBER as u32 + 64 - 1) / 64; // One workgroup per chamber's particles
                    cpass.dispatch_workgroups(workgroups, 1, 1);
                }
                queue.submit(Some(encoder.finish()));
            }
            let _ = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(std::time::Duration::from_secs(1)) });

            // Read back and calculate fitness
            let buffer_size = (total_particles * std::mem::size_of::<GpuMagneticParticle>()) as wgpu::BufferAddress;
            let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Staging Buffer"), size: buffer_size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            encoder.copy_buffer_to_buffer(&particle_buffer, 0, &staging_buffer, 0, buffer_size);
            queue.submit(Some(encoder.finish()));
            let _ = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(std::time::Duration::from_secs(1)) });

            let buffer_slice = staging_buffer.slice(..);
            let (sender, receiver) = tokio::sync::oneshot::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
            device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(std::time::Duration::from_secs(1)) }); // Poll for map_async
            let _ = tokio::time::timeout(std::time::Duration::from_secs(1), receiver).await.unwrap().unwrap();

            let data = buffer_slice.get_mapped_range();
            let final_particles: &[GpuMagneticParticle] = bytemuck::cast_slice(&data);

            // Sum flow for this genome's particles
            let mut total_flow_for_genome = 0.0;
            // Iterate only over the particles belonging to the current genome
            let start_offset = genome_idx * PARTICLES_PER_CHAMBER;
            for p in final_particles.iter().skip(start_offset).take(PARTICLES_PER_CHAMBER) {
                total_flow_for_genome += p.properties[0]; // properties[0] stores current_flow_accum
            }
            fitness_scores[genome_idx] = total_flow_for_genome;

            drop(data);
            staging_buffer.unmap();
        }

        // --- Selection, Crossover, Mutation ---
        let mut sorted_indices: Vec<usize> = (0..POPULATION_SIZE).collect();
        sorted_indices.sort_by(|&a, &b| fitness_scores[b].partial_cmp(&fitness_scores[a]).unwrap()); // Descending order

        println!("Generation {} Results:", generation_num);
        for i in 0..POPULATION_SIZE {
            let idx = sorted_indices[i];
            println!("  Genome {}: Flow = {:.4}", idx, fitness_scores[idx]);
        }
        
        // Update best genome found so far
        if fitness_scores[sorted_indices[0]] > best_fitness {
            best_fitness = fitness_scores[sorted_indices[0]];
            best_genome = population[sorted_indices[0]].clone();
        }


        if generation_num < GENERATIONS - 1 {
            let mut new_population: Vec<Genome> = Vec::with_capacity(POPULATION_SIZE);
            let num_parents = POPULATION_SIZE / 2; // Take top half as parents

            for i in 0..POPULATION_SIZE {
                if i < num_parents {
                    // Keep the top parents
                    new_population.push(population[sorted_indices[i]].clone());
                } else {
                    // Crossover and mutate
                    let parent_a_idx = sorted_indices[rng.gen_range(0..num_parents)];
                    let parent_b_idx = sorted_indices[rng.gen_range(0..num_parents)];
                    let mut child = Genome::crossover(&population[parent_a_idx], &population[parent_b_idx]);
                    child = child.mutate(MUTATION_RATE, MUTATION_STRENGTH);
                    new_population.push(child);
                }
            }
            population = new_population;
        }
    }

    println!("\nEvolution Complete.");
    println!("Best Genome found: {:?}", best_genome);
    println!("With flow: {:.4}", best_fitness);
}