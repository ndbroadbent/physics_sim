use physics_sim::gpu_data::{GpuParticle, SimParams};
use physics_sim::experiments::{ExperimentConfig, ExperimentResult};
use physics_sim::elements::{Element, MaterialDef};
use wgpu::util::DeviceExt;
use std::time::Instant;

    // Constants for simulation
const SIM_WIDTH: u32 = 800;
const SIM_HEIGHT: u32 = 200;
const PHOTON_INITIAL_ENERGY_EV: f64 = 2.5;
const NUM_PARTICLES: usize = 100_000; // Reduced particle count for faster prototyping
const WORKGROUP_SIZE: u32 = 64;

async fn run_gpu_simulation(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    config: &ExperimentConfig,
) -> ExperimentResult {
    println!("Running experiment: {}", config.name);

    let mut initial_data = Vec::with_capacity(config.photon_count);
    for _ in 0..config.photon_count {
        let y = rand::random::<f32>() * config.sim_height as f32; 
        initial_data.push(GpuParticle::new_photon(0.0, y, config.photon_energy_ev as f32));
    }

    let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Particle Buffer"),
        contents: bytemuck::cast_slice(&initial_data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
    });

    let sim_params = SimParams {
        width: config.sim_width as f32,
        height: config.sim_height as f32,
        atom_density: config.atom_density as f32,
        band_gap: config.host_material.band_gap as f32,
        dt: config.dt,
        padding: [0.0, 0.0, 0.0],
    };
    let param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Param Buffer"),
        contents: bytemuck::cast_slice(&[sim_params]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

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

    // 3. Simulation Loop
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
    
    // Timeout for simulation
    let poll_timeout = std::time::Duration::from_secs(2);
    let _ = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(poll_timeout) });
    
    let duration = start_time.elapsed();
    println!("  Simulated {} steps for {} particles in {:.2?}", config.sim_steps, config.photon_count, duration);

    // 4. Read back results
    println!("  Reading back results...");
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
    
    // Wait for copy
    let _ = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(poll_timeout) });

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
    
    // Ensure we poll so the map_async callback fires!
    // On some backends, you must poll for map_async to complete.
    device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(poll_timeout) });

    // Wait for mapping with timeout
    match tokio::time::timeout(std::time::Duration::from_secs(2), receiver).await {
        Ok(Ok(Ok(()))) => {},
        Ok(Ok(Err(e))) => panic!("Buffer map failed: {:?}", e),
        Ok(Err(_)) => panic!("Sender dropped"),
        Err(_) => panic!("Timeout waiting for buffer map"),
    }

    let data = buffer_slice.get_mapped_range();
    let particles: &[GpuParticle] = bytemuck::cast_slice(&data);

    let mut absorbed_photons = 0;
    let mut transmitted_photons = 0;

    for p in particles {
        let p_type = p.properties[1];
        if p_type == GpuParticle::TYPE_PHOTON {
            if p.properties[2] < 0.5 { // Inactive Photon
                absorbed_photons += 1;
            } else {
                transmitted_photons += 1;
            }
        }
    }
    drop(data); 
    staging_buffer.unmap();    
    let collected_pairs = absorbed_photons as f64 * 0.8;
    let estimated_voc = config.host_material.band_gap * 0.7;
    let output_energy_proxy_ev = collected_pairs * estimated_voc;
    let total_input_photon_energy_ev = config.photon_count as f64 * config.photon_energy_ev;
    let efficiency_percent = (output_energy_proxy_ev / total_input_photon_energy_ev) * 100.0;

    ExperimentResult {
        config_name: config.name.clone(),
        material_band_gap: config.host_material.band_gap,
        estimated_voc,
        collected_pairs,
        efficiency_percent,
        absorbed_photons,
        transmitted_photons,
    }
}

#[tokio::main] 
async fn main() {
    env_logger::init();
    println!("Initializing GPU Experiment Framework...");

    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Failed to find a suitable GPU adapter");
    println!("Selected GPU: {:?}", adapter.get_info());

    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            ..Default::default()
        },
    ).await.expect("Failed to create device");

    let experiments = vec![
        ExperimentConfig {
            name: "Silicon Standard".to_string(),
            host_material: MaterialDef::silicon(),
            n_dopant: Some(Element::Phosphorus),
            p_dopant: Some(Element::Boron),
            n_doping_concentration: 0.001,
            p_doping_concentration: 0.001,
            photon_energy_ev: PHOTON_INITIAL_ENERGY_EV,
            photon_count: NUM_PARTICLES,
            sim_steps: 800,
            dt: 1.0,
            sim_width: SIM_WIDTH,
            sim_height: SIM_HEIGHT,
            atom_density: 0.1,
        },
        ExperimentConfig {
            name: "Gallium Arsenide (GaAs)".to_string(),
            host_material: MaterialDef::gallium_arsenide(),
            n_dopant: Some(Element::Sulfur),
            p_dopant: Some(Element::Zinc),
            n_doping_concentration: 0.001,
            p_doping_concentration: 0.001,
            photon_energy_ev: PHOTON_INITIAL_ENERGY_EV,
            photon_count: NUM_PARTICLES,
            sim_steps: 800,
            dt: 1.0,
            sim_width: SIM_WIDTH,
            sim_height: SIM_HEIGHT,
            atom_density: 0.1,
        },
        ExperimentConfig {
            name: "Germanium (Ge)".to_string(),
            host_material: MaterialDef::germanium(),
            n_dopant: Some(Element::Phosphorus),
            p_dopant: Some(Element::Boron),
            n_doping_concentration: 0.001,
            p_doping_concentration: 0.001,
            photon_energy_ev: PHOTON_INITIAL_ENERGY_EV,
            photon_count: NUM_PARTICLES,
            sim_steps: 800,
            dt: 1.0,
            sim_width: SIM_WIDTH,
            sim_height: SIM_HEIGHT,
            atom_density: 0.1,
        },
    ];

    let mut results = Vec::new();
    let total_start = Instant::now();
    let time_limit = std::time::Duration::from_secs(15);

    for config in &experiments {
        if total_start.elapsed() > time_limit {
            println!("Global time limit of 15s reached. Stopping experiments.");
            break;
        }
        
        let result = run_gpu_simulation(&device, &queue, config).await;
        results.push(result);
    }

    println!("\n--- Experiment Results ---");
    for res in &results {
        println!("{}: Efficiency = {:.2}% (Absorbed: {}, Transmitted: {})",
            res.config_name, res.efficiency_percent, res.absorbed_photons, res.transmitted_photons);
        println!("    Est. Voc: {:.2} V, Collected Pairs: {:.1}", res.estimated_voc, res.collected_pairs);
    }

    if let Err(e) = physics_sim::experiments::plot_results(&results, "experiment_comparison.png") {
        eprintln!("Error plotting results: {}", e);
    } else {
        println!("\nComparison plot saved to 'experiment_comparison.png'");
    }
}