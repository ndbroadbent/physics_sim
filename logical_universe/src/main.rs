use clap::Parser;
use std::borrow::Cow;
use wgpu::util::DeviceExt;
use image::{ImageBuffer, Rgb};
use std::path::Path;
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use std::process::{Command, Stdio};
use std::io::Write;
use std::fs::File;
use std::io::BufWriter;

mod gpu_data;
use gpu_data::SimParams;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = String::from("visualize"))]
    mode: String,

    #[arg(long, default_value_t = 100)]
    modulus: u32,

    #[arg(long, default_value_t = 1)]
    sustain_threshold: u32,

    #[arg(long, default_value_t = 10000)]
    total_frames: u32,

    #[arg(long, default_value_t = false)]
    enable_ffmpeg: bool,

    #[arg(long, default_value_t = true)]
    enable_image_dump: bool, 

    #[arg(long, default_value_t = false)]
    enable_data_dump: bool,

    #[arg(long, default_value_t = false)]
    no_wrap: bool,

    #[arg(long, default_value_t = 50)]
    exp_start_modulus: u32,

    #[arg(long, default_value_t = 300)]
    exp_end_modulus: u32,

    #[arg(long, default_value_t = 1)]
    exp_start_threshold: u32,

    #[arg(long, default_value_t = 3)]
    exp_end_threshold: u32,

    #[arg(long, default_value_t = 20000)]
    exp_max_frames: u32,

    #[arg(long, default_value_t = 1)]
    vis_stride: u32,
}

const DIM_X: u32 = 128;
const DIM_Y: u32 = 128;
const DIM_Z: u32 = 128;
const IMG_W: u32 = 256;
const IMG_H: u32 = 256;

async fn run_simulation(args: Args) {
    env_logger::init();
    std::fs::create_dir_all("frames").expect("Failed to create frames directory");

    let font_path = "/System/Library/Fonts/Monaco.ttf";
    let font_data = std::fs::read(font_path).expect("Failed to load font");
    let font = Font::try_from_bytes(&font_data).expect("Error constructing Font");

    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
        },
        None,
    ).await.unwrap();

    let vol_size = (DIM_X * DIM_Y * DIM_Z) as usize;
    let buffer_size = (vol_size * std::mem::size_of::<u32>()) as u64;

    let get_initial_data = || {
        let mut top_data = vec![0u32; vol_size];
        let mut bottom_data = vec![1u32; vol_size];
        {
            let cx = DIM_X / 2;
            let cy = DIM_Y / 2;
            let cz = DIM_Z / 2;
            let idx = (cz * DIM_Y * DIM_X + cy * DIM_X + cx) as usize;
            top_data[idx] = 1;
        }
        {
            let cx2 = 12;
            let cy2 = 42;
            let cz2 = 7;
            let idx2 = (cz2 * DIM_Y * DIM_X + cy2 * DIM_X + cx2) as usize;
            top_data[idx2] = 1;
        }
        (top_data, bottom_data)
    };

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
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
    });

    let top_buf_a = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Top A"), size: buffer_size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let top_buf_b = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Top B"), size: buffer_size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let bottom_buf_a = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Bot A"), size: buffer_size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let bottom_buf_b = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Bot B"), size: buffer_size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Staging"), size: buffer_size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let params_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Params"), size: std::mem::size_of::<SimParams>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

    if args.mode == "experiment" {
        for modulus in args.exp_start_modulus..=args.exp_end_modulus {
            for sustain_threshold in args.exp_start_threshold..=args.exp_end_threshold {
                let (init_top, init_bottom) = get_initial_data();
                queue.write_buffer(&top_buf_a, 0, bytemuck::cast_slice(&init_top));
                queue.write_buffer(&bottom_buf_a, 0, bytemuck::cast_slice(&init_bottom));
                
                let mut top_ops = 0;
                let mut bottom_ops = 0;
                
                // CSV Setup
                let csv_filename = format!("frames/telemetry_m{}_s{}.csv", modulus, sustain_threshold);
                let file = File::create(&csv_filename).expect("Failed to create CSV file");
                let mut writer = BufWriter::new(file);
                writeln!(writer, "Modulus,Threshold,Frame,TopMass,BottomZeros,TotalActive").expect("Failed to write CSV header");

                let mut survival_frame = args.exp_max_frames;

                for frame in 0..args.exp_max_frames {
                    // Wiggle
                    let mut state = (frame as u32).wrapping_add(123456789);
                    state ^= state << 13; state ^= state >> 17; state ^= state << 5;
                    let ox = (state % 3) as i32 - 1;
                    let oy = ((state >> 2) % 3) as i32 - 1;
                    let oz = ((state >> 4) % 3) as i32 - 1;
                    
                    // let axis = (state >> 6) % 2;
                    let step_type = if ((state >> 6) % 2) == 0 {
                        let op = if (top_ops % modulus) < sustain_threshold { 5 } else { 2 };
                        top_ops += 1; op
                    } else {
                        let op = if (bottom_ops % modulus) < sustain_threshold { 4 } else { 3 };
                        bottom_ops += 1; op
                    };

                    let params = SimParams { 
                        width: DIM_X, height: DIM_Y, depth: DIM_Z, 
                        offset_x: ox, offset_y: oy, offset_z: oz, 
                        step_type, frame: frame as u32,
                        wrap: if args.no_wrap { 0 } else { 1 },
                        _pad: [0; 3],
                    };
                    queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

                    let (top_in, top_out, bottom_in, bottom_out) = if frame % 2 == 0 { (&top_buf_a, &top_buf_b, &bottom_buf_a, &bottom_buf_b) } else { (&top_buf_b, &top_buf_a, &bottom_buf_b, &bottom_buf_a) };

                    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: None, layout: &bind_group_layout,
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
                    
                    if frame % 5 == 0 { // Check more frequently (every 5 frames)
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
                        let top_mass: u32 = top_slice.iter().sum();
                        
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
                        let bottom_zeros: u32 = bottom_slice.iter().map(|&x| 1u32 - x).sum();
                        let total_active = top_mass + bottom_zeros;

                        // Write to CSV (every 5 frames)
                        writeln!(writer, "{},{},{},{},{},{}", modulus, sustain_threshold, frame, top_mass, bottom_zeros, total_active).unwrap();

                        // Print to stdout (every 100 frames)
                        if frame % 100 == 0 {
                            println!("M:{} T:{} F:{} Active:{}", modulus, sustain_threshold, frame, total_active);
                        }

                        if total_active == 0 {
                            survival_frame = frame;
                            break;
                        }
                    } else {
                        queue.submit(Some(encoder.finish()));
                    }
                }
                println!("Modulus {} Threshold {} finished. Survival: {}", modulus, sustain_threshold, survival_frame);
            }
        }
    } else { 
        // VISUALIZE MODE
        let (init_top, init_bottom) = get_initial_data();
        queue.write_buffer(&top_buf_a, 0, bytemuck::cast_slice(&init_top));
        queue.write_buffer(&bottom_buf_a, 0, bytemuck::cast_slice(&init_bottom));
        
        let mut top_ops = 0;
        let mut bottom_ops = 0;
        
        let mut ffmpeg_in = if args.enable_ffmpeg {
             let mut child = Command::new("ffmpeg")
                .args(&["-y", "-f", "rawvideo", "-pixel_format", "rgb24", "-video_size", &format!("{}x{}", IMG_W, IMG_H), "-framerate", "30", "-i", "-", "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p", "logical_universe.mkv"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("Failed to start ffmpeg");
             Some(child.stdin.take().expect("Failed to open ffmpeg stdin"))
        } else {
            None
        };

        for frame in 0..args.total_frames {
            let mut state = (frame as u32).wrapping_add(123456789);
            state ^= state << 13; state ^= state >> 17; state ^= state << 5;
            let ox = (state % 3) as i32 - 1;
            let oy = ((state >> 2) % 3) as i32 - 1;
            let oz = ((state >> 4) % 3) as i32 - 1;
            // let axis = (state >> 6) % 2;

            let (step_type, op_name) = if ((state >> 6) % 2) == 0 {
                let op = if (top_ops % args.modulus) < args.sustain_threshold { 5 } else { 2 };
                let name = if op == 5 { "XNOR" } else { "NOR" };
                top_ops += 1; (op, name)
            } else {
                let op = if (bottom_ops % args.modulus) < args.sustain_threshold { 4 } else { 3 };
                let name = if op == 4 { "XOR" } else { "NAND" };
                bottom_ops += 1; (op, name)
            };

            let params = SimParams { 
                width: DIM_X, height: DIM_Y, depth: DIM_Z, 
                offset_x: ox, offset_y: oy, offset_z: oz, 
                step_type, frame: frame as u32,
                wrap: if args.no_wrap { 0 } else { 1 },
                _pad: [0; 3],
            };
            queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));
            let (top_in, top_out, bottom_in, bottom_out) = if frame % 2 == 0 { (&top_buf_a, &top_buf_b, &bottom_buf_a, &bottom_buf_b) } else { (&top_buf_b, &top_buf_a, &bottom_buf_b, &bottom_buf_a) };

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None, layout: &bind_group_layout,
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
            
            // Readback for Dump/Vis
            
            if frame % args.vis_stride == 0 { 
                encoder.copy_buffer_to_buffer(top_out, 0, &staging_buffer, 0, buffer_size);
                queue.submit(Some(encoder.finish())); // Submit compute + copy
                
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

                // DUMP individual data dumps if enabled
                if args.enable_data_dump {
                     let filename = format!("frames/dump_{}.bin", frame);
                     let mut file = File::create(&filename).expect("create failed");
                     let mut writer = BufWriter::new(file);
                     for &val in &top_slice { writer.write_all(&val.to_le_bytes()).unwrap(); }
                     for &val in &bottom_slice { writer.write_all(&val.to_le_bytes()).unwrap(); }
                     println!("Saved {}", filename);
                }

                // IMAGE RENDERING (ALWAYS RUNS IF vis_stride hit)
                let mut img = ImageBuffer::new(IMG_W, IMG_H);
                let center = [DIM_X as f32 / 2.0, DIM_Y as f32 / 2.0, DIM_Z as f32 / 2.0];
                let radius = DIM_X as f32 * 0.9;
                let angle = (frame as f32 * 0.05) * 0.5;
                let cam_y = center[1] + radius * 0.3;
                let cam_x = center[0] + radius * angle.cos();
                let cam_z = center[2] + radius * angle.sin();
                let cam_pos = [cam_x, cam_y, cam_z];
                let fwd = normalize(sub(center, cam_pos));
                let right = normalize(cross(fwd, [0.0, 1.0, 0.0]));
                let up = cross(right, fwd);

                for py in 0..IMG_H {
                    for px in 0..IMG_W {
                        let uv_x = (px as f32 / IMG_W as f32) * 2.0 - 1.0;
                        let uv_y = 1.0 - (py as f32 / IMG_H as f32) * 2.0;
                        let ray_dir = normalize(add(fwd, add(scale(right, uv_x), scale(up, uv_y))));
                        let (t_min, t_max) = intersect_box(cam_pos, ray_dir, [0.0,0.0,0.0], [DIM_X as f32, DIM_Y as f32, DIM_Z as f32]);
                        
                        let mut r = 0.0; let mut g = 0.0; let mut b = 0.0; let mut alpha_acc = 0.0;
                        if t_min < t_max && t_max > 0.0 {
                            let mut t = t_min.max(0.0);
                            while t < t_max && alpha_acc < 1.0 {
                                let p = add(cam_pos, scale(ray_dir, t));
                                let ix = p[0] as u32; let iy = p[1] as u32; let iz = p[2] as u32;
                                if ix < DIM_X && iy < DIM_Y && iz < DIM_Z {
                                     let idx = (iz * DIM_Y * DIM_X + iy * DIM_X + ix) as usize;
                                     let val_t = top_slice[idx]; let val_b = bottom_slice[idx];
                                     let (cr, cg, cb, a) = match (val_t, val_b) {
                                        (0, 1) => (0.0, 0.0, 0.0, 0.0),
                                        (0, 0) => (0.5, 0.0, 0.5, 0.15),
                                        (1, 1) => (1.0, 1.0, 1.0, 0.3),
                                        (1, 0) => (0.0, 1.0, 1.0, 0.2),
                                        _ => (0.0, 0.0, 0.0, 0.0),
                                     };
                                     if a > 0.0 {
                                         let contrib = a * (1.0 - alpha_acc);
                                         r += cr*contrib; g += cg*contrib; b += cb*contrib; alpha_acc += contrib;
                                     }
                                }
                                t += 1.0;
                            }
                        }
                        img.put_pixel(px, py, Rgb([(r*255.0) as u8, (g*255.0) as u8, (b*255.0) as u8]));
                    }
                }
                
                let debug_text = format!("T={:04} | Op:{}", frame, op_name);
                draw_text_mut(&mut img, Rgb([255, 255, 0]), 10, 10, Scale::uniform(20.0), &font, &debug_text);

                // DUMP individual images if enabled AND ffmpeg not enabled (redundant otherwise)
                if args.enable_image_dump && !args.enable_ffmpeg {
                    img.save(format!("frames/frame_{:05}.png", frame)).unwrap();
                    println!("Saved image frames/frame_{:05}.png", frame);
                }
                
                // FEED TO FFMPEG if enabled
                if args.enable_ffmpeg { // Explicitly check enable_ffmpeg for piping
                    if let Some(ref mut pipe) = ffmpeg_in {
                        pipe.write_all(&img.into_raw()).unwrap(); // img.into_raw() directly pipes pixels
                    }
                }
            } else {
                queue.submit(Some(encoder.finish()));
            }
        }
    }
}

// Vector Math Helpers (Copied)
fn normalize(v: [f32; 3]) -> [f32; 3] { let len = (v[0]*v[0] + v[1]*v[1] + v[2]*v[2]).sqrt(); if len == 0.0 { return [0.0, 0.0, 0.0]; } [v[0]/len, v[1]/len, v[2]/len] }
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0]-b[0], a[1]-b[1], a[2]-b[2]] }
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0]+b[0], a[1]+b[1], a[2]+b[2]] }
fn scale(v: [f32; 3], s: f32) -> [f32; 3] { [v[0]*s, v[1]*s, v[2]*s] }
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[1]*b[2] - a[2]*b[1], a[2]*b[0] - a[0]*b[2], a[0]*b[1] - a[1]*b[0]] }
fn intersect_box(origin: [f32; 3], dir: [f32; 3], box_min: [f32; 3], box_max: [f32; 3]) -> (f32, f32) {
    let mut t_min: f32 = -1e30; let mut t_max: f32 = 1e30;
    for i in 0..3 {
        if dir[i] != 0.0 {
            let t1 = (box_min[i] - origin[i]) / dir[i];
            let t2 = (box_max[i] - origin[i]) / dir[i];
            t_min = t_min.max(t1.min(t2)); t_max = t_max.min(t1.max(t2));
        } else if origin[i] < box_min[i] || origin[i] > box_max[i] { return (1e30, -1e30); }
    }
    (t_min, t_max)
}

fn main() { pollster::block_on(run_simulation(Args::parse())); }