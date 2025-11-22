use physics_sim::gpu_data::{GpuParticle, SimParams};
use physics_sim::experiments::{ExperimentConfig, ExperimentResult, LayerDef};
use physics_sim::elements::{Element, MaterialDef};
use wgpu::util::DeviceExt;
use std::time::Instant;

// Constants
const SIM_WIDTH: u32 = 800;
const SIM_HEIGHT: u32 = 200;
const PHOTON_INITIAL_ENERGY_EV: f64 = 2.5; // Blue-ish light
const NUM_PARTICLES: usize = 100_000;
const WORKGROUP_SIZE: u32 = 64;

async fn run_gpu_simulation(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    config: &ExperimentConfig,
) -> ExperimentResult {
    println!("Running experiment: {}", config.name);

    // 1. Prepare Particles
    let mut initial_data = Vec::with_capacity(config.photon_count);
    for _ in 0..config.photon_count {
        let y = rand::random::<f32>() * config.sim_height as f32; 
        // Spectrum: For multi-junction, we need broad spectrum!
        // Let's randomize energy between 0.5 eV (IR) and 3.5 eV (UV)
        let energy = 0.5 + rand::random::<f32>() * 3.0; 
        initial_data.push(GpuParticle::new_photon(0.0, y, energy));
    }

    let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Particle Buffer"),
        contents: bytemuck::cast_slice(&initial_data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
    });

    // 2. Map Layers to SimParams
    let mut sim_params = SimParams {
        width: config.sim_width as f32,
        height: config.sim_height as f32,
        atom_density: config.atom_density as f32,
        dt: config.dt,
        
        l1_start: 0.0, l1_end: 0.0, l1_band_gap: 0.0, l1_active: 0.0,
        l2_start: 0.0, l2_end: 0.0, l2_band_gap: 0.0, l2_active: 0.0,
        l3_start: 0.0, l3_end: 0.0, l3_band_gap: 0.0, l3_active: 0.0,
        padding: [0.0, 0.0, 0.0, 0.0],
    };

    let mut current_x = 200.0; // Start materials at x=200
    for (i, layer) in config.layers.iter().enumerate() {
        let end_x = current_x + layer.width;
        if i == 0 {
            sim_params.l1_start = current_x; sim_params.l1_end = end_x;
            sim_params.l1_band_gap = layer.material.band_gap as f32; sim_params.l1_active = 1.0;
        } else if i == 1 {
            sim_params.l2_start = current_x; sim_params.l2_end = end_x;
            sim_params.l2_band_gap = layer.material.band_gap as f32; sim_params.l2_active = 1.0;
        } else if i == 2 {
            sim_params.l3_start = current_x; sim_params.l3_end = end_x;
            sim_params.l3_band_gap = layer.material.band_gap as f32; sim_params.l3_active = 1.0;
        }
        current_x = end_x;
    }

    let param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Param Buffer"),
        contents: bytemuck::cast_slice(&[sim_params]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // 3. Pipeline Setup
    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
        ],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: param_buffer.as_entire_binding(),
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(), 
        cache: None,
    });

    // 4. Run Simulation
    let start_time = Instant::now();
    for _ in 0..config.sim_steps {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            cpass.set_pipeline(&compute_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (config.photon_count as u32 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            cpass.dispatch_workgroups(workgroups, 1, 1);
        }
        queue.submit(Some(encoder.finish()));
    }
    
    // Timeout logic
    let poll_timeout = std::time::Duration::from_secs(2);
    let _ = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(poll_timeout) });
    
    // 5. Read Results
    let buffer_size = (config.photon_count * std::mem::size_of::<GpuParticle>()) as wgpu::BufferAddress;
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    encoder.copy_buffer_to_buffer(&particle_buffer, 0, &staging_buffer, 0, buffer_size);
    queue.submit(Some(encoder.finish()));
    let _ = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(poll_timeout) });

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
    
    device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(poll_timeout) });
    if let Err(_) = tokio::time::timeout(poll_timeout, receiver).await {
        panic!("GPU Timeout reading buffer");
    }

    let data = buffer_slice.get_mapped_range();
    let particles: &[GpuParticle] = bytemuck::cast_slice(&data);

    let mut layer_absorbed = vec![0u32; 4]; // 0=none, 1=L1, 2=L2, 3=L3
    let mut transmitted = 0;
    let mut total_energy_collected = 0.0;
    let mut total_input_energy = 0.0;

    for p in particles {
        // Reconstruct initial energy roughly or track it? GpuParticle stores energy in properties[0]
        let initial_energy = p.properties[0] as f64;
        total_input_energy += initial_energy;

        if p.properties[2] < 0.5 { // Absorbed
            let layer_idx = p.properties[3] as usize; // w component holds layer index
            if layer_idx > 0 && layer_idx <= 3 {
                layer_absorbed[layer_idx] += 1;
                
                // Calculate energy collected from this specific photon
                // E_out = Voc * q ~ BandGap * 0.7
                let band_gap = match layer_idx {
                    1 => sim_params.l1_band_gap,
                    2 => sim_params.l2_band_gap,
                    3 => sim_params.l3_band_gap,
                    _ => 0.0,
                } as f64;
                
                total_energy_collected += band_gap * 0.7; 
            }
        } else {
            transmitted += 1;
        }
    }
    drop(data);
    staging_buffer.unmap();

    let efficiency = (total_energy_collected / total_input_energy) * 100.0;

    ExperimentResult {
        config_name: config.name.clone(),
        layers_info: format!("{} Layers", config.layers.len()),
        total_efficiency_percent: efficiency,
        absorbed_counts: layer_absorbed,
        transmitted_photons: transmitted,
    }
}

#[tokio::main] async fn main() {
    env_logger::init();
    
    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::Performance,
        ..Default::default()
    }).await.unwrap();

    let experiments = vec![
        // 1. Standard Silicon (Single Junction)
        ExperimentConfig {
            name: "Standard Silicon".to_string(),
            layers: vec![
                LayerDef { material: MaterialDef::silicon(), width: 400.0 },
            ],
            photon_energy_ev: 0.0, // Random spectrum handled in run_gpu_simulation
            photon_count: NUM_PARTICLES,
            sim_steps: 800, dt: 1.0, sim_width: SIM_WIDTH, sim_height: SIM_HEIGHT, atom_density: 0.1,
        },
        // 2. Dual Junction (GaAs on Silicon)
        // GaAs (1.42 eV) absorbs high energy blue light
        // Silicon (1.12 eV) absorbs the rest
        ExperimentConfig {
            name: "Tandem GaAs/Si".to_string(),
            layers: vec![
                LayerDef { material: MaterialDef::gallium_arsenide(), width: 200.0 },
                LayerDef { material: MaterialDef::silicon(), width: 200.0 },
            ],
            photon_energy_ev: 0.0,
            photon_count: NUM_PARTICLES,
            sim_steps: 800, dt: 1.0, sim_width: SIM_WIDTH, sim_height: SIM_HEIGHT, atom_density: 0.1,
        },
        // 3. Triple Junction (Theoretical)
        // High (2.0 eV) -> Med (1.4 eV) -> Low (0.7 eV)
        ExperimentConfig {
            name: "Triple Junction".to_string(),
            layers: vec![
                LayerDef { material: MaterialDef { name: "High-Gap".into(), band_gap: 2.0, elements: vec![] }, width: 130.0 },
                LayerDef { material: MaterialDef::gallium_arsenide(), width: 130.0 },
                LayerDef { material: MaterialDef::germanium(), width: 140.0 },
            ],
            photon_energy_ev: 0.0,
            photon_count: NUM_PARTICLES,
            sim_steps: 800, dt: 1.0, sim_width: SIM_WIDTH, sim_height: SIM_HEIGHT, atom_density: 0.1,
        },
    ];

    let mut results = Vec::new();
    for config in experiments {
        results.push(run_gpu_simulation(&device, &queue, &config).await);
    }

    println!("\n--- Multi-Junction Results ---");
    for res in &results {
        println!("{}: Efficiency = {:.2}%", res.config_name, res.total_efficiency_percent);
        println!("   Layer Absorption: L1:{}, L2:{}, L3:{}", 
            res.absorbed_counts[1], res.absorbed_counts[2], res.absorbed_counts[3]);
    }
    
    physics_sim::experiments::plot_results(&results, "multijunction_comparison.png").unwrap();
    println!("\nSaved 'multijunction_comparison.png'");
}
