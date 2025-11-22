use anyhow::Result;
use wgpu::util::DeviceExt;
use image::{ImageBuffer, Rgba};

// Parameters
const GRID_WIDTH: u32 = 1024; // Increased width
const GRID_HEIGHT: u32 = 1024; // Increased height
const DT: f32 = 0.1;
const DX: f32 = 1.0;
const DAMP: f32 = 0.9995; // Less damping
const STEPS_PER_FRAME: u32 = 10;
const TOTAL_FRAMES: u32 = 700;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct WaveParams {
    width: u32,
    height: u32,
    dt: f32,
    dx: f32,
    damp: f32,
    padding: [f32; 3], // Align to 16 bytes
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // 1. Setup GPU
    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default()).await.unwrap();

    // 2. Initialize Grid Data
    let grid_size = (GRID_WIDTH * GRID_HEIGHT) as usize;
    let mut u_field = vec![0.0f32; grid_size];
    let mut u_prev = vec![0.0f32; grid_size];
    let mut material = vec![[1.0f32, 0.0, 0.0, 0.0]; grid_size]; // n0=1.0 (Air), n2=0.0

    // --- Setup Scene: Dynamic Double Slit ---
    let wall_x_center = GRID_WIDTH / 2;
    let wall_width = 10;
    let wall_x_start = wall_x_center - wall_width / 2;
    let wall_x_end = wall_x_center + wall_width / 2;

    let slit_height = 30; // 30 pixels high
    let slit_gap = 50; // Gap between slits

    let slit_y_center = GRID_HEIGHT / 2;
    let slit1_y_start = slit_y_center - slit_gap / 2 - slit_height;
    let slit1_y_end = slit_y_center - slit_gap / 2;
    let slit2_y_start = slit_y_center + slit_gap / 2;
    let slit2_y_end = slit_y_center + slit_gap / 2 + slit_height;

    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let idx = (y * GRID_WIDTH + x) as usize;

            // Material: Wall
            if x >= wall_x_start && x <= wall_x_end {
                if (y >= slit1_y_start && y < slit1_y_end) || (y >= slit2_y_start && y < slit2_y_end) {
                    // Slit (Air)
                    material[idx] = [1.0, 0.0, 0.0, 0.0];
                } else {
                    // Wall (High Index / Block)
                    material[idx] = [3.0, 0.0, 0.0, 0.0];
                }
            }

            // Non-Linear Kerr Material Region after the slits
            // Fixed size: 150 pixels wide, 110 pixels high
            if x > wall_x_end + 50 && x < wall_x_end + 50 + 150 && y > (GRID_HEIGHT/2) - 55 && y < (GRID_HEIGHT/2) + 55 {
                // n0=1.5, n2=0.5 (Strong non-linearity)
                material[idx] = [1.5, 0.5, 0.0, 0.0];
            }
        }
    }

    // Initial Pulse (Gaussian) at Left, vertically centered
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let idx = (y * GRID_WIDTH + x) as usize;
            let dx = x as f32 - (GRID_WIDTH as f32 / 4.0); // 1/4 of the way in
            let dy = y as f32 - (GRID_HEIGHT as f32 / 2.0); // Centered
            let dist = (dx*dx + dy*dy).sqrt();
            if dist < 20.0 {
                u_field[idx] = (-dist * 0.1).exp() * 10.0; // High amplitude pulse
                u_prev[idx] = u_field[idx]; // Stationary start (splits into 2 waves)
            }
        }
    }

    // 3. Create Buffers
    let buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Buffer A"), contents: bytemuck::cast_slice(&u_field), usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Buffer B"), contents: bytemuck::cast_slice(&u_prev), usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
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
    println!("Simulating Wave Propagation...");

    for _frame in 0..TOTAL_FRAMES {
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
                let wgs = (GRID_WIDTH + 15) / 16;
                cpass.dispatch_workgroups(wgs, wgs, 1);
            }
        }

        queue.submit(Some(encoder.finish()));
        // Polling is implicit or we wait at the end
    }

    // Wait for all frames
    device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();

    // 6. Readback Last Frame
    let source_buffer = if current_bind_group == 0 { &buffer_a } else { &buffer_b };

    let buffer_size = (grid_size * 4) as wgpu::BufferAddress; // f32
    let buffer_size_mat = (grid_size * 16) as wgpu::BufferAddress; // vec4

    let staging = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: buffer_size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let staging_mat = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: buffer_size_mat, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    encoder.copy_buffer_to_buffer(source_buffer, 0, &staging, 0, buffer_size);
    encoder.copy_buffer_to_buffer(&mat_buffer, 0, &staging_mat, 0, buffer_size_mat);
    queue.submit(Some(encoder.finish()));

    let (tx, rx) = tokio::sync::oneshot::channel();
    let (tx2, rx2) = tokio::sync::oneshot::channel();

    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());

    let slice_mat = staging_mat.slice(..);
    slice_mat.map_async(wgpu::MapMode::Read, move |v| tx2.send(v).unwrap());

    device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();
    rx.await.unwrap().unwrap();
    rx2.await.unwrap().unwrap();

    let data = slice.get_mapped_range();
    let floats: &[f32] = bytemuck::cast_slice(&data);

    let data_mat = slice_mat.get_mapped_range();
    let mats: &[f32] = bytemuck::cast_slice(&data_mat); // stride 4

    // Save Image
    let mut img = ImageBuffer::new(GRID_WIDTH, GRID_HEIGHT);
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let idx = (y * GRID_WIDTH + x) as usize;
            let val = floats[idx];

                        // Material Visualization

                        let mat_idx = idx * 4;

                        let n0 = mats[mat_idx];



                        // Base color (Material)

                        let mut r = 0u8;

                        let mut g = 0u8;

                        let mut b = 0u8;



                        if n0 > 1.1 {

                            let wall_val = if n0 > 4.0 { 100 } else { 50 }; // Light gray for wall, Dark for glass

                            r = wall_val;

                            g = wall_val;

                            b = wall_val;

                        }



                        // Add Wave (Additive blending)

                        let intensity = (val.abs() * 20.0).clamp(0.0, 1.0); // Boosted brightness



                        if val < 0.0 {

                            let wave_r = (intensity * 255.0) as u8;

                            r = r.saturating_add(wave_r);

                        } else {

                            let wave_b = (intensity * 255.0) as u8;

                            b = b.saturating_add(wave_b);

                        }



                        // Green channel for "energy" intensity or just keep material gray

                        if val.abs() > 0.01 {

                             let wave_g = (intensity * 50.0) as u8;

                             g = g.saturating_add(wave_g);

                        }



                        img.put_pixel(x, y, Rgba([r, g, b, 255]));


        }
    }

    img.save("wave_output.png").unwrap();
    println!("Saved 'wave_output.png'");

    Ok(())
}
