use std::borrow::Cow;
use wgpu::util::DeviceExt;
use image::{ImageBuffer, Rgb};
use std::path::Path;
use imageproc::drawing::draw_text_mut;
use ab_glyph::{FontRef, PxScale};
use std::process::{Command, Stdio};
use std::io::Write;

mod gpu_data;
use gpu_data::SimParams;

// 3D Dimensions
const DIM_X: u32 = 128;
const DIM_Y: u32 = 128;
const DIM_Z: u32 = 128;

// Output Image Res
const IMG_W: u32 = 256;
const IMG_H: u32 = 256;

async fn run() {
    env_logger::init();

    // Load Font
    let font_path = "/System/Library/Fonts/Monaco.ttf";
    let font_data = std::fs::read(font_path).expect("Failed to load font");
    let font = FontRef::try_from_slice(&font_data).expect("Error constructing Font");

    // Initialize FFMPEG Process
    let mut ffmpeg = Command::new("ffmpeg")
        .args(&[
            "-y", // Overwrite output
            "-f", "rawvideo",
            "-pixel_format", "rgb24",
            "-video_size", &format!("{}x{}", IMG_W, IMG_H),
            "-framerate", "30",
            "-i", "-", // Input from stdin
            "-c:v", "libx264",
            "-preset", "fast", // Fast encoding for real-time feel
            "-crf", "18", // High quality
            "-pix_fmt", "yuv420p",
            "logical_universe.mkv" // MKV container for robustness
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null()) // Quiet stdout
        .stderr(Stdio::inherit()) // Show stderr for progress/errors
        .spawn()
        .expect("Failed to start ffmpeg");

    let mut ffmpeg_in = ffmpeg.stdin.take().expect("Failed to open ffmpeg stdin");

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
    let vol_size = (DIM_X * DIM_Y * DIM_Z) as usize;
    let mut top_data = vec![0u32; vol_size];
    let mut bottom_data = vec![1u32; vol_size];

    // Seeds (Center)
    {
        let cx = DIM_X / 2;
        let cy = DIM_Y / 2;
        let cz = DIM_Z / 2;
        let idx = (cz * DIM_Y * DIM_X + cy * DIM_X + cx) as usize;

        top_data[idx] = 1;
        // Bottom layer remains all 1s (stable vacuum)
        println!("Initialized seed in Top layer at ({}, {}, {})", cx, cy, cz);
    }

    // Second Top Seed (1) at (12, 42, 7)
    {
        let cx2 = 12;
        let cy2 = 42;
        let cz2 = 7;
        let idx2 = (cz2 * DIM_Y * DIM_X + cy2 * DIM_X + cx2) as usize;
        top_data[idx2] = 1;
        println!("Initialized second seed in Top layer at ({}, {}, {})", cx2, cy2, cz2);
    }

    let buffer_size = (vol_size * std::mem::size_of::<u32>()) as u64;

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
    let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Params Buffer"),
        size: std::mem::size_of::<SimParams>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let total_frames = 10000;
    let mut top_ops = 0;
    let mut bottom_ops = 0;

    for frame in 0..total_frames {
        // Xorshift Chaotic Wiggle
        let mut state = (frame as u32).wrapping_add(123456789);
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;

        let ox = (state % 3) as i32 - 1; // -1, 0, 1
        let oy = ((state >> 2) % 3) as i32 - 1;
        let oz = ((state >> 4) % 3) as i32 - 1;
        let axis = (state >> 6) % 2; // 0 or 1

        // Logic Ops (Standard Model)
        let (step_type, op_name) = if axis == 0 {
            let (op, name) = match top_ops % 2 {
                0 => (5, "XNOR"), // Sustains Seed
                _ => (2, "NOR")   // Carves
            };
            top_ops += 1;
            (op, name)
        } else {
            let (op, name) = match bottom_ops % 2 {
                0 => (4, "XOR"),  // Sustains Seed
                _ => (3, "NAND")  // Carves
            };
            bottom_ops += 1;
            (op, name)
        };

        let params = SimParams {
            width: DIM_X, height: DIM_Y, depth: DIM_Z,
            offset_x: ox, offset_y: oy, offset_z: oz,
            step_type, frame: frame as u32,
        };
        queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

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
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            cpass.set_pipeline(&pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(DIM_X / 4, DIM_Y / 4, DIM_Z / 4);
        }

        // Readback
        encoder.copy_buffer_to_buffer(top_out, 0, &staging_buffer, 0, buffer_size);
        queue.submit(Some(encoder.finish()));
        let top_slice = {
            let buffer_slice = staging_buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
            device.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();
            let data = buffer_slice.get_mapped_range();
            let res: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
            drop(data);
            staging_buffer.unmap();
            res
        };

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
            let res: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
            drop(data);
            staging_buffer.unmap();
            res
        };

        // --- Orbiting Camera Raycaster ---
        let mut img = ImageBuffer::new(IMG_W, IMG_H);

        // Camera Setup
        let center = [DIM_X as f32 / 2.0, DIM_Y as f32 / 2.0, DIM_Z as f32 / 2.0];
        let radius = DIM_X as f32 * 0.9; // Zoomed in
        let angle = (frame as f32 * 0.05) * 0.5; // Rotate
        let cam_y = center[1] + radius * 0.3; // Slightly above
        let cam_x = center[0] + radius * angle.cos();
        let cam_z = center[2] + radius * angle.sin();
        let cam_pos = [cam_x, cam_y, cam_z];

        // Basis Vectors (LookAt)
        let fwd = normalize(sub(center, cam_pos));
        let world_up = [0.0, 1.0, 0.0];
        let right = normalize(cross(fwd, world_up));
        let up = cross(right, fwd);

        // Render
        for py in 0..IMG_H {
            for px in 0..IMG_W {
                let uv_x = (px as f32 / IMG_W as f32) * 2.0 - 1.0;
                let uv_y = 1.0 - (py as f32 / IMG_H as f32) * 2.0;

                let ray_dir = normalize(add(fwd, add(scale(right, uv_x), scale(up, uv_y))));

                let (t_min, t_max) = intersect_box(cam_pos, ray_dir, [0.0, 0.0, 0.0], [DIM_X as f32, DIM_Y as f32, DIM_Z as f32]);

                let mut r = 0.0;
                let mut g = 0.0;
                let mut b = 0.0;
                let mut alpha_acc = 0.0;

                if t_min < t_max && t_max > 0.0 {
                    let start_t = t_min.max(0.0);
                    let end_t = t_max;
                    let step_size = 1.0;
                    let mut t = start_t;

                    while t < end_t && alpha_acc < 1.0 {
                        let p = add(cam_pos, scale(ray_dir, t));
                        let ix = p[0] as u32;
                        let iy = p[1] as u32;
                        let iz = p[2] as u32;

                        if ix < DIM_X && iy < DIM_Y && iz < DIM_Z {
                            let idx = (iz * DIM_Y * DIM_X + iy * DIM_X + ix) as usize;
                            let val_t = top_slice[idx];
                            let val_b = bottom_slice[idx];

                            let (cr, cg, cb, a) = match (val_t, val_b) {
                                (0, 1) => (0.0, 0.0, 0.0, 0.0),
                                (0, 0) => (0.5, 0.0, 0.5, 0.15),
                                (1, 1) => (1.0, 1.0, 1.0, 0.3),
                                (1, 0) => (0.0, 1.0, 1.0, 0.2),
                                _ => (0.0, 0.0, 0.0, 0.0),
                            };

                            if a > 0.0 {
                                let contrib = a * (1.0 - alpha_acc);
                                r += cr * contrib;
                                g += cg * contrib;
                                b += cb * contrib;
                                alpha_acc += contrib;
                            }
                        }
                        t += step_size;
                    }
                }

                img.put_pixel(px, py, Rgb([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]));
            }
        }

        // Draw Debug Text
        let layer_name = if axis == 0 { "TOP" } else { "BOT" };
        let debug_text = format!("T={:04} | L:{: <3} | Op:{: <9}", frame, layer_name, op_name);

        let scale = PxScale::from(20.0);
        draw_text_mut(&mut img, Rgb([255, 255, 0]), 10, 10, scale, &font, &debug_text);

        // Write frame to FFMPEG
        ffmpeg_in.write_all(&img).unwrap();

        if frame % 10 == 0 {
            let top_ones: u32 = top_slice.iter().sum();
            let bottom_zeros: u32 = bottom_slice.iter().map(|&x| 1u32 - x).sum();
            println!("Frame {}: Top Ones = {}, Bottom Zeros = {}", frame, top_ones, bottom_zeros);
        }
    }
    println!("Done!");
}

// Vector Math Helpers
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0]*v[0] + v[1]*v[1] + v[2]*v[2]).sqrt();
    if len == 0.0 { return [0.0, 0.0, 0.0]; }
    [v[0]/len, v[1]/len, v[2]/len]
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0]-b[0], a[1]-b[1], a[2]-b[2]] }
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0]+b[0], a[1]+b[1], a[2]+b[2]] }
fn scale(v: [f32; 3], s: f32) -> [f32; 3] { [v[0]*s, v[1]*s, v[2]*s] }
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1]*b[2] - a[2]*b[1], a[2]*b[0] - a[0]*b[2], a[0]*b[1] - a[1]*b[0]]
}
fn intersect_box(origin: [f32; 3], dir: [f32; 3], box_min: [f32; 3], box_max: [f32; 3]) -> (f32, f32) {
    let mut t_min: f32 = -1e30;
    let mut t_max: f32 = 1e30;
    for i in 0..3 {
        if dir[i] != 0.0 {
            let t1 = (box_min[i] - origin[i]) / dir[i];
            let t2 = (box_max[i] - origin[i]) / dir[i];
            t_min = t_min.max(t1.min(t2));
            t_max = t_max.min(t1.max(t2));
        } else if origin[i] < box_min[i] || origin[i] > box_max[i] {
            return (1e30, -1e30);
        }
    }
    (t_min, t_max)
}

fn main() { pollster::block_on(run()); }
