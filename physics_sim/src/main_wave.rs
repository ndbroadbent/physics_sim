use anyhow::Result;
use wgpu::util::DeviceExt;
use image::{ImageBuffer, Rgba};
use std::fs;

// Parameters
const GRID_WIDTH: u32 = 1280;
const GRID_HEIGHT: u32 = 720;
const DT: f32 = 0.1;
const DX: f32 = 1.0;
const DAMP: f32 = 1.0; // Lossless
const STEPS_PER_FRAME: u32 = 5; 
const TOTAL_FRAMES: u32 = 8000; // Increased duration for Soliton propagation
const SAVE_INTERVAL: u32 = 1; // Save every frame

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct WaveParams {
    width: u32,
    height: u32,
    dt: f32,
    dx: f32,
    damp: f32,
    padding: [f32; 3],
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("CWD: {:?}", std::env::current_dir()?);
    fs::create_dir_all("frames")?; 

    // 1. Setup GPU
    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default()).await.unwrap();

    // 2. Initialize Grid Data
    let grid_size = (GRID_WIDTH * GRID_HEIGHT) as usize;
    let mut u_field = vec![0.0f32; grid_size];
    let mut u_prev = vec![0.0f32; grid_size];
    let mut material = vec![[1.0f32, 0.0, 0.0, 0.0]; grid_size]; 

    // --- Setup Scene: Full Grid Uniform Non-Linear Kerr Medium ---
    let n0_kerr = 1.5; // Base refractive index (glass)
    let n2_kerr = 0.0001; // Non-linear coefficient (tuned for soliton)

    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let idx = (y * GRID_WIDTH + x) as usize;
            material[idx] = [n0_kerr, n2_kerr, 0.0, 0.0]; // Entire grid is Kerr medium
        }
    }
    
    // Initial Pulse: A Gaussian Beam launched from the left
    let beam_amplitude = 50.0; // Crucial parameter for soliton formation
    let beam_width = 15.0; // Controls diffraction
    let beam_start_x = GRID_WIDTH as f32 / 8.0; // Start 1/8th of the way in

    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let idx = (y * GRID_WIDTH + x) as usize;
            
            let dx = x as f32 - beam_start_x;
            let dy = y as f32 - (GRID_HEIGHT as f32 / 2.0); // Vertically centered
            
            // Gaussian profile in Y
            let gaussian_y = (-dy*dy / (2.0 * beam_width*beam_width)).exp();
            
            // Gaussian profile in X (Pulse packet width)
            let pulse_length = 40.0;
            let gaussian_x = (-dx*dx / (2.0 * pulse_length*pulse_length)).exp();
            
            // Initial pulse shape (Packet)
            u_field[idx] = beam_amplitude * gaussian_y * gaussian_x;
            
            // u_prev(x) = u_field(x - v*dt) => shift center of gaussian_x
            // Center was beam_start_x. New center is beam_start_x - (c*dt).
            // dx_prev = x - (beam_start_x - c*dt) = dx + c*dt
            let prev_dx = dx + (1.0 * DT);
            let gaussian_x_prev = (-prev_dx*prev_dx / (2.0 * pulse_length*pulse_length)).exp();
            
            u_prev[idx] = beam_amplitude * gaussian_y * gaussian_x_prev;
        }
    }

    // 3. Create Buffers
    let buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Buffer A"), contents: bytemuck::cast_slice(&u_field), usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
    });
    let buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Buffer B"), contents: bytemuck::cast_slice(&u_prev), usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
    });
    let mat_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Material"), contents: bytemuck::cast_slice(&material), usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    
    let params = WaveParams { width: GRID_WIDTH, height: GRID_HEIGHT, dt: DT, dx: DX, damp: DAMP, padding: [0.0; 3] };
    let param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params"), contents: bytemuck::cast_slice(&[params]), usage: wgpu::BufferUsages::UNIFORM,
    });

    // 4. Pipeline
    let shader = device.create_shader_module(wgpu::include_wgsl!("wave.wgsl"));
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
        ],
    });

    // We need two bind groups for Ping-Pong
    // BG0: A -> Current, B -> Next
    // BG1: B -> Current, A -> Next
    let bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None, layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: buffer_a.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: buffer_b.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: mat_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: param_buffer.as_entire_binding() },
        ],
    });
    let bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None, layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: buffer_b.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: buffer_a.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: mat_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: param_buffer.as_entire_binding() },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&bind_group_layout], push_constant_ranges: &[] });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None, layout: Some(&pipeline_layout), module: &shader, entry_point: Some("main"), compilation_options: Default::default(), cache: None,
    });

    // 5. Run Loop
    let mut current_bind_group = 0;
    println!("Simulating Soliton Propagation ({} frames)...", TOTAL_FRAMES);

    let buffer_size = (grid_size * 4) as wgpu::BufferAddress;
    let buffer_size_mat = (grid_size * 16) as wgpu::BufferAddress;
    let staging = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: buffer_size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let staging_mat = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: buffer_size_mat, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

    for frame in 0..TOTAL_FRAMES {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            cpass.set_pipeline(&pipeline);
            
            for _ in 0..STEPS_PER_FRAME {
                if current_bind_group == 0 {
                    cpass.set_bind_group(0, &bind_group_0, &[]);
                    current_bind_group = 1;
                } else {
                    cpass.set_bind_group(0, &bind_group_1, &[]);
                    current_bind_group = 0;
                }
                let wgs_x = (GRID_WIDTH + 15) / 16;
                let wgs_y = (GRID_HEIGHT + 15) / 16;
                cpass.dispatch_workgroups(wgs_x, wgs_y, 1);
            }
        }
        
        if frame % SAVE_INTERVAL == 0 {
            let source_buffer = if current_bind_group == 0 { &buffer_a } else { &buffer_b };
            encoder.copy_buffer_to_buffer(source_buffer, 0, &staging, 0, buffer_size);
            encoder.copy_buffer_to_buffer(&mat_buffer, 0, &staging_mat, 0, buffer_size_mat);
        }
        
        queue.submit(Some(encoder.finish()));
        
        if frame % SAVE_INTERVAL == 0 {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let (tx2, rx2) = tokio::sync::oneshot::channel();

            let slice = staging.slice(..);
            slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
            
            let slice_mat = staging_mat.slice(..);
            slice_mat.map_async(wgpu::MapMode::Read, move |v| tx2.send(v).unwrap());

            device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();
            rx.await.unwrap().unwrap();
            rx2.await.unwrap().unwrap();
            
            {
                let data = slice.get_mapped_range();
                let floats: &[f32] = bytemuck::cast_slice(&data);
                
                let data_mat = slice_mat.get_mapped_range();
                let mats: &[f32] = bytemuck::cast_slice(&data_mat); 
                
                let mut img = ImageBuffer::new(GRID_WIDTH, GRID_HEIGHT);
                for y in 0..GRID_HEIGHT {
                    for x in 0..GRID_WIDTH {
                        let idx = (y * GRID_WIDTH + x) as usize;
                        let val = floats[idx];
                        
                        let mat_idx = idx * 4;
                        let n0 = mats[mat_idx];
                        
                        let mut r = 0u8;
                        let mut g = 0u8;
                        let mut b = 0u8;
                        
                        if n0 > 1.1 {
                            // Draw the uniform Kerr medium background as a light gray
                            r = 100; g = 100; b = 100;
                        }

                        let intensity = (val.abs() * 20.0).clamp(0.0, 1.0); 
                        if val < 0.0 { 
                            let wave_r = (intensity * 255.0) as u8;
                            r = r.saturating_add(wave_r);
                        } else { 
                            let wave_b = (intensity * 255.0) as u8;
                            b = b.saturating_add(wave_b);
                        }
                        if val.abs() > 0.01 {
                             let wave_g = (intensity * 50.0) as u8;
                             g = g.saturating_add(wave_g);
                        }
                        img.put_pixel(x, y, Rgba([r, g, b, 255]));
                    }
                }
                let filename = format!("frames/frame_{:04}.png", frame / SAVE_INTERVAL);
                img.save(&filename).unwrap();
                if frame % 100 == 0 {
                    println!("Saved {}", filename);
                }
            }
            staging.unmap();
            staging_mat.unmap();
        }
    }
    
    println!("Simulation Complete. Frames saved to 'frames/'.");
    
    Ok(())
}