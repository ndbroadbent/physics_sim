use std::borrow::Cow;
use wgpu::util::DeviceExt;
use image::{ImageBuffer, Rgb};
use std::path::Path;

mod gpu_data;
use gpu_data::SimParams;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 1024;
const WORKGROUP_SIZE: u32 = 16;

async fn run() {
    env_logger::init();

    // 1. Initialize GPU
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .unwrap();

    // 2. Initialize Data
    let mut top_data = vec![0u32; (WIDTH * HEIGHT) as usize];
    let mut bottom_data = vec![1u32; (WIDTH * HEIGHT) as usize];

    // Flip one random bit in Top layer (0 -> 1)
    {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let cx = WIDTH / 2;
        let cy = HEIGHT / 2;
        let idx = (cy * WIDTH + cx) as usize;
        top_data[idx] = 1;
        println!("Initialized random bit in Top layer at ({}, {})", cx, cy);
    }
    
    // Flip one random bit in Bottom layer (1 -> 0)
    {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let cx = WIDTH / 2;
        let cy = HEIGHT / 2;
        let idx = (cy * WIDTH + cx) as usize;
        bottom_data[idx] = 0;
        println!("Initialized random bit in Bottom layer at ({}, {})", cx, cy);
    }
    
    // Create Buffers (Ping-Pong: A -> B -> A)
    // We need 4 buffers total: Top A, Top B, Bottom A, Bottom B
    let buffer_size = (top_data.len() * std::mem::size_of::<u32>()) as u64;
    
    let top_buf_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Top Buffer A"),
        contents: bytemuck::cast_slice(&top_data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
    });
    let top_buf_b = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Top Buffer B"),
        size: buffer_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    
    let bottom_buf_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Bottom Buffer A"),
        contents: bytemuck::cast_slice(&bottom_data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
    });
    let bottom_buf_b = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Bottom Buffer B"),
        size: buffer_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // 3. Pipeline Setup
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind Group Layout"),
        entries: &[
            // Params
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Top In
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Top Out
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Bottom In
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
             // Bottom Out
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
        compilation_options: Default::default(),
    });

    // 4. Simulation Loop
    let mut offset_x = 0;
    let mut offset_y = 0;
    
    // We create the param buffer once and update it? Or create new one every time?
    // Updating is better.
    let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Params Buffer"),
        size: std::mem::size_of::<SimParams>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Ensure frames directory exists
    let frames_dir = "frames";
    if !Path::new(frames_dir).exists() {
        std::fs::create_dir(frames_dir).unwrap();
    }

    // Movement Cycle: (offset_x, offset_y, axis)
    // axis: 0 = Top (Vertical), 1 = Bottom (Horizontal)
    let moves = [
        // 2x2 Cycle (8 steps)
        (0, 1, 0), (0, 2, 0), // Up
        (1, 2, 1), (2, 2, 1), // Right
        (2, 1, 0), (2, 0, 0), // Down
        (1, 0, 1), (0, 0, 1), // Left
    ];
    
    let total_frames = 1000;
    let mut top_ops = 0;
    let mut bottom_ops = 0;
    
    for frame in 0..total_frames {
        let step = frame % 8;
        let (ox, oy, axis) = moves[step as usize];
        offset_x = ox;
        offset_y = oy;

        let step_type = if axis == 0 {
            // Top Layer: Sequence AND, NOR, XNOR
            let op = match top_ops % 3 {
                0 => 0, // AND
                1 => 2, // NOR
                _ => 5, // XNOR
            };
            top_ops += 1;
            op
        } else {
            // Bottom Layer: Sequence OR, NAND, XOR
            let op = match bottom_ops % 3 {
                0 => 1, // OR
                1 => 3, // NAND
                _ => 4, // XOR
            };
            bottom_ops += 1;
            op
        };
        
        // Update Params
        let params = SimParams {
            width: WIDTH,
            height: HEIGHT,
            offset_x,
            offset_y,
            step_type,
            frame: frame as u32,
            _padding: [0; 2],
        };
        queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));
        
        // Determine Input/Output buffers (Ping-Pong)
        // Even frame: A -> B
        // Odd frame: B -> A
        let (top_in, top_out, bottom_in, bottom_out) = if frame % 2 == 0 {
            (&top_buf_a, &top_buf_b, &bottom_buf_a, &bottom_buf_b)
        } else {
            (&top_buf_b, &top_buf_a, &bottom_buf_b, &bottom_buf_a)
        };

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: top_in.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: top_out.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: bottom_in.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: bottom_out.as_entire_binding() },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                 label: None,
                 timestamp_writes: None, 
            });
            cpass.set_pipeline(&pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }

        // Visualization: Read back data to generate image
        // We need both Top and Bottom to sum them.
        // Since we just computed 'out' buffers, those are the current state.
        // However, we can only map one buffer at a time efficiently or need multiple copies.
        // Let's copy TopOut and BottomOut to CPU. 
        // Wait, MapRead requires the buffer to be MAP_READ. Storage buffers usually aren't.
        // We copy Storage -> Staging (Mapped).
        
        // We need TWO staging buffers or copy sequentially.
        // Let's copy Top -> Staging, read, then Bottom -> Staging, read.
        // This is slow but fine for offline rendering.
        
        encoder.copy_buffer_to_buffer(top_out, 0, &staging_buffer, 0, buffer_size);
        queue.submit(Some(encoder.finish()));
        
        // Read Top
        let top_slice = {
            let buffer_slice = staging_buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
            device.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();
            let data = buffer_slice.get_mapped_range();
            let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
            drop(data);
            staging_buffer.unmap();
            result
        };

        // Read Bottom
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(bottom_out, 0, &staging_buffer, 0, buffer_size);
        queue.submit(Some(encoder.finish()));
        
        let bottom_slice = {
            let buffer_slice = staging_buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
            device.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();
            let data = buffer_slice.get_mapped_range();
            let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
            drop(data);
            staging_buffer.unmap();
            result
        };

        // Generate Image
        let mut img = ImageBuffer::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let idx = (y * WIDTH + x) as usize;
                let t = top_slice[idx];
                let b = bottom_slice[idx];
                
                let pixel = match (t, b) {
                    (0, 0) => Rgb([0u8, 0u8, 0u8]),       // Black
                    (1, 0) => Rgb([0u8, 0u8, 139u8]),     // Dark Blue (Top=1, Bottom=0)
                    (0, 1) => Rgb([30u8, 144u8, 255u8]),  // Medium Blue (Top=0, Bottom=1)
                    (1, 1) => Rgb([255u8, 255u8, 255u8]), // White
                    _ => Rgb([255u8, 0u8, 0u8]),          // Error
                };
                img.put_pixel(x, y, pixel);
            }
        }
        img.save(format!("{}/frame_{:05}.png", frames_dir, frame)).unwrap();
        
        // Log every frame now.
        if frame % 100 == 0 || true { // Force log every frame for debugging
            let top_count: u32 = top_slice.iter().sum();
            let bottom_count: u32 = bottom_slice.iter().sum();
            
            let mut coords = String::new();
            if top_count > 0 && top_count < 20 {
                let mut points = Vec::new();
                for (i, &val) in top_slice.iter().enumerate() {
                    if val != 0 {
                        let y = i as u32 / WIDTH;
                        let x = i as u32 % WIDTH;
                        points.push(format!("({}, {})", x, y));
                    }
                }
                coords = format!(" [{}]", points.join(", "));
            }
            
            println!("Frame {}: Top Ones = {}{}, Bottom Ones = {}", frame, top_count, coords, bottom_count);
        }
    }
    
    println!("Done! Run ffmpeg to generate video.");
}

fn main() {
    pollster::block_on(run());
}