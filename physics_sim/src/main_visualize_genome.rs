use anyhow::{anyhow, Result};
use wgpu::util::DeviceExt;
// use physics_sim::genetic::Genome; // Unused
use image::{ImageBuffer, Rgba};

// Uniforms struct (Rust side) must match WGSL
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    resolution: [f32; 2],
    time: f32,
    _pad1: f32, // Align to 16 bytes
    camera_pos: [f32; 3],
    _pad2: f32, // Align vec3 to 16 bytes
    camera_target: [f32; 3],
    _pad3: f32, // Align vec3 to 16 bytes
    camera_up: [f32; 3],
    _pad4: f32, // Align vec3 to 16 bytes
    genome_params: [f32; 16], 
}

async fn run_headless() -> Result<()> {
    // Hardcoded best genome from the previous run
    let best_genome_genes: [f32; 16] = [
        0.7704699, 0.006599307, 0.14029002, 0.77480626, 0.14399093, 
        0.6627429, 0.73306084, 0.1451807, 0.106321275, 0.851491, 
        0.41654706, 0.87280214, 0.9913798, 0.35109216, 0.08714211, 0.8621229
    ];
    
    // Dimensions for the output image
    let width = 1920;
    let height = 1080;

    // 1. Setup WGPU (Headless)
    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Failed to find an appropriate adapter"); // Simply expect the Option/Result to be valid

    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            ..Default::default()
        },
    ).await?;

    // 2. Create Output Texture
    let texture_desc = wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: Some("Output Texture"),
        view_formats: &[],
    };
    let texture = device.create_texture(&texture_desc);
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // 3. Create Output Buffer (to read back to CPU)
    // Align width to 256 bytes for buffer copy
    let u32_size = std::mem::size_of::<u32>() as u32;
    let align = 256;
    let unpadded_bytes_per_row = width * 4;
    let padding = (align - unpadded_bytes_per_row % align) % align;
    let padded_bytes_per_row = unpadded_bytes_per_row + padding;
    let buffer_size = (padded_bytes_per_row * height) as wgpu::BufferAddress;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // 4. Setup Shader and Uniforms
    let shader = device.create_shader_module(wgpu::include_wgsl!("render_shader.wgsl"));

    // Camera setup: Look at the "Block" from a distance
    // Move camera back to z=40, y=20 to look down at the 20x20x20 block
    let uniforms_data = Uniforms {
        resolution: [width as f32, height as f32],
        time: 0.0,
        _pad1: 0.0,
        camera_pos: [40.0, 30.0, 60.0], // Pulled back and up
        _pad2: 0.0,
        camera_target: [0.0, 0.0, 0.0], // Look at center
        _pad3: 0.0,
        camera_up: [0.0, 1.0, 0.0],
        _pad4: 0.0,
        genome_params: best_genome_genes,
    };

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&[uniforms_data]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Uniform Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
        label: Some("Uniform Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }
        ],
    });

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: texture_desc.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    // 5. Render
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.05, g: 0.05, b: 0.1, a: 1.0, // Dark background
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }

    // 6. Copy to Buffer
    // Check if we need TexelCopy or ImageCopy based on version. 
    // Assuming wgpu 27 has TexelCopy based on previous error.
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        texture_desc.size,
    );

    queue.submit(Some(encoder.finish()));

    // 7. Map and Save
    let buffer_slice = output_buffer.slice(..);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
    
    device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();
    receiver.await.unwrap().unwrap();

    let data = buffer_slice.get_mapped_range();
    
    // Save to PNG
    let mut img_buf = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, vec![0u8; (width * height * 4) as usize]).unwrap();
    
    for y in 0..height {
        for x in 0..width {
            let buffer_index = (y * padded_bytes_per_row + x * 4) as usize;
            // Copy row by row to handle padding if necessary
            if buffer_index < data.len() {
                let r = data[buffer_index];
                let g = data[buffer_index + 1];
                let b = data[buffer_index + 2];
                let a = data[buffer_index + 3];
                img_buf.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }
    }

    img_buf.save("genome_visualization.png")?;
    println!("Saved 'genome_visualization.png'");

    drop(data);
    output_buffer.unmap();

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    if let Err(e) = run_headless().await {
        eprintln!("Error: {}", e);
    }
}