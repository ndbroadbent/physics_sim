use physics_sim::shader_gen::{SdfOp, TransformOp};
use wgpu::util::DeviceExt;
use rand::Rng;
use std::time::Instant;

// --- Parameters ---
const POPULATION_SIZE: usize = 30;
const GENERATIONS: usize = 50;
const ELITISM_COUNT: usize = 3;

// FDTD Grid
const GRID_WIDTH: u32 = 128;
const GRID_HEIGHT: u32 = 128;
const SIM_STEPS: u32 = 400; // Time for wave to stabilize/cross
const DT: f32 = 0.5; // Stability limit is 0.707
const DX: f32 = 1.0;

// Logic Targets (NAND)
// Inputs: (Bias, A, B). Target Output.
// Bias is implicitly always 1.
// A, B -> Out
const TEST_CASES: &[(f32, f32, f32)] = &[
    (0.0, 0.0, 1.0), // 0 NAND 0 = 1 (Signal via Bias)
    (0.0, 1.0, 1.0), // 0 NAND 1 = 1
    (1.0, 0.0, 1.0), // 1 NAND 0 = 1
    (1.0, 1.0, 0.0), // 1 NAND 1 = 0 (Destructive Interference)
];

// Detector Locations (Quorum of 3 on the right side)
// Grid is 128x128.
// Right edge ~ 120. Center Y ~ 64.
// Spread them out a bit.
const DETECTORS: &[(u32, u32)] = &[
    (110, 64),      // Center
    (110, 64 + 8),  // Top
    (110, 64 - 8),  // Bottom
];

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SimParams {
    width: u32,
    height: u32,
    dt: f32,
    dx: f32,
    input_a_active: f32,
    input_b_active: f32,
    bias_active: f32,
    time: f32,
}

// --- Shader Templates ---

const SHADER_HELPERS: &str = r#"
// Helper functions for Geometry and Symmetry
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

// Standard SDF Ops
fn opUnion(d1: f32, d2: f32) -> f32 { return min(d1, d2); }
fn opSubtract(d1: f32, d2: f32) -> f32 { return max(d1, -d2); }
fn opIntersect(d1: f32, d2: f32) -> f32 { return max(d1, d2); }

fn opSmoothUnion(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp( 0.5 + 0.5*(d2-d1)/k, 0.0, 1.0 );
    return mix( d2, d1, h ) - k*h*(1.0-h);
}
fn opSmoothSubtract(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp( 0.5 - 0.5*(d2+d1)/k, 0.0, 1.0 );
    return mix( d2, -d1, h ) + k*h*(1.0-h);
}
fn opSmoothIntersect(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp( 0.5 - 0.5*(d2-d1)/k, 0.0, 1.0 );
    return mix( d2, d1, h ) + k*h*(1.0-h);
}

// Transform Ops (for SdfOp::Transform)
fn opTwist(p: vec3<f32>, k: f32) -> vec3<f32> {
    let c = cos(k*p.y);
    let s = sin(k*p.y);
    let m = mat2x2<f32>(c, -s, s, c);
    return vec3<f32>(m * p.xz, p.y);
}

fn opFold(p: vec3<f32>, k: vec3<f32>) -> vec3<f32> {
    return abs(p) - k;
}

// Repeat (for SdfOp::Repeat)
fn opRep(p: vec3<f32>, c: f32) -> vec3<f32> {
    return (p % c) - (c * 0.5);
}


// Kaleidoscope: 6-fold symmetry around Z-axis (Screen Center)
fn kaleidoscope(p: vec3<f32>) -> vec3<f32> {
    let segments = 6.0; // 6-fold symmetry
    let angle_step = 6.2831853 / segments; // 2*PI / segments
    
    let r = length(p.xy);
    let a = atan2(p.y, p.x);
    
    // Fold the angle into the first sector (0 to angle_step)
    let a_folded = (a % angle_step + angle_step) % angle_step; // Ensure positive modulo
    
    // Mirror within the sector
    let a_mirror = abs(a_folded - angle_step * 0.5); 
    
    // Reconstruct p in the folded sector
    return vec3<f32>(r * cos(a_mirror), r * sin(a_mirror), p.z);
}
"#;

const SHADER_TEMPLATE: &str = r#"
// FDTD Logic Gate Evolution Shader

// --- GEOMETRY PLACEHOLDER ---
// The Rust code will inject the helper functions and the map_geometry function here.
// INSERT_HELPERS_HERE

fn map_geometry_raw(p: vec3<f32>) -> f32 {
    // INSERT_GENERATED_CODE_HERE
    return 1.0; 
}

fn map_geometry(p: vec3<f32>) -> f32 {
    // Apply Kaleidoscope symmetry before evaluating geometry
    let p_sym = kaleidoscope(p);
    return map_geometry_raw(p_sym);
}

// --- FDTD Simulation ---

struct SimParams {
    width: u32,
    height: u32,
    dt: f32,
    dx: f32,
    input_a_active: f32, // 1.0 or 0.0
    input_b_active: f32, // 1.0 or 0.0
    bias_active: f32,    // Always 1.0
    time: f32,           // Current simulation time
};

@group(0) @binding(0) var<storage, read_write> u_current: array<f32>;
@group(0) @binding(1) var<storage, read_write> u_prev: array<f32>;
@group(0) @binding(2) var<uniform> params: SimParams;

fn idx(x: u32, y: u32) -> u32 {
    return y * params.width + x;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x <= 0u || x >= params.width - 1u || y <= 0u || y >= params.height - 1u) {
        // Simple Absorbing Boundary (Damping layer)
        if (x < 5u || x > params.width - 5u || y < 5u || y > params.height - 5u) {
             let i = idx(x, y);
             u_prev[i] = u_current[i] * 0.8; // Strong damp
        }
        return;
    }

    let i = idx(x, y);
    let u_c = u_current[i];
    let u_p = u_prev[i];

    // Laplacian
    let laplacian = u_current[idx(x+1u, y)] + u_current[idx(x-1u, y)] +
                    u_current[idx(x, y+1u)] + u_current[idx(x, y-1u)] - 
                    4.0 * u_c;

    // Geometry mapping
    let aspect = f32(params.width) / f32(params.height);
    // Map 0..Width to -10..+10
    let uv = vec2<f32>(f32(x) / f32(params.width), f32(y) / f32(params.height));
    let p_coord = vec3<f32>((uv.x * 2.0 - 1.0) * 10.0 * aspect, (uv.y * 2.0 - 1.0) * 10.0, 0.0);
    
    let dist = map_geometry(p_coord);
    
    // Material properties
    var n = 1.0; // Air
    // Soft boundary? Or hard?
    if (dist < 0.0) {
        n = 1.5; // Glass
    }
    
    // Kerr Effect (Non-linear) - Crucial for interaction
    let intensity = u_c * u_c;
    let n2 = 0.05; // Strong non-linearity
    n = n + n2 * intensity;

    let c = 1.0 / n;
    let courant = (c * params.dt / params.dx);
    let courant_sq = courant * courant;

    var u_next = 2.0 * u_c - u_p + courant_sq * laplacian;
    
    // Damping
    u_next *= 0.999; // Minimal loss

    // --- Inputs ---
    // Continuous sine waves at specific ports
    let freq = 0.3; 
    let val = sin(params.time * freq);
    
    // Bias Input (Top Left)
    if (params.bias_active > 0.5 && x == 10u && y == 32u) { u_next = val; }
    
    // Input A (Left Middle)
    if (params.input_a_active > 0.5 && x == 10u && y == 64u) { u_next = val; }
    
    // Input B (Left Bottom)
    if (params.input_b_active > 0.5 && x == 10u && y == 96u) { u_next = val; }

    // Write to prev (Ping-Pong logic: we bind prev as output)
    u_prev[i] = u_next;
}
"#;

#[tokio::main]
async fn main() {
    env_logger::init();
    println!("Initializing Logic Gate Evolution (NAND + Kaleidoscope)...");

    // 1. Setup GPU
    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default()).await.unwrap();

    // 2. Population Init
    let mut population: Vec<SdfOp> = Vec::with_capacity(POPULATION_SIZE);
    // Seed with an "empty" geometry (always air) to ensure light can pass
    population.push(SdfOp::Scalar(1.0)); // Always returns 1.0, so dist > 0 -> n=1.0 (air)
    for _ in 1..POPULATION_SIZE {
        population.push(SdfOp::random(5));
    }
    
    // Shared Buffers (re-used for every individual to save VRAM)
    let grid_size = (GRID_WIDTH * GRID_HEIGHT) as usize;
    let grid_bytes = (grid_size * 4) as u64;
    
    let buffer_a = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Buffer A"), size: grid_bytes, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false
    });
    let buffer_b = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Buffer B"), size: grid_bytes, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false
    });
    // We need a readback buffer
    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Readback"), size: grid_bytes, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false
    });
    
    // Param buffer
    let param_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Params"), size: std::mem::size_of::<SimParams>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
        ],
    });

    // Create Bind Groups (Ping Pong)
    let bg_a_to_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None, layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: buffer_a.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: buffer_b.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: param_buffer.as_entire_binding() },
        ],
    });
    let bg_b_to_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None, layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: buffer_b.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: buffer_a.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: param_buffer.as_entire_binding() },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&bind_group_layout], push_constant_ranges: &[] });

    let mut rng = rand::rng();

    for generation_num in 0..GENERATIONS {
        let start_time = Instant::now();
        let mut fitness_scores: Vec<(usize, f32)> = Vec::new();

        // Evaluate Population
        for (idx, genome) in population.iter().enumerate() {
            // 1. Compile
            let geom_code = genome.to_wgsl("p");
            let full_source = SHADER_TEMPLATE
                .replace("// INSERT_HELPERS_HERE", SHADER_HELPERS)
                .replace("// INSERT_GENERATED_CODE_HERE", &format!("    return {};", geom_code));

            let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None, source: wgpu::ShaderSource::Wgsl(full_source.into()),
            });

            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None, layout: Some(&pipeline_layout), module: &shader_module, entry_point: Some("main"), compilation_options: Default::default(), cache: None,
            });

            let mut total_error = 0.0;

            // 2. Test Cases (NAND Truth Table)
            for (in_a, in_b, target) in TEST_CASES {
                // Clear Buffers (Important!)
                // We can't easily clear storage buffers with a command unless we have a clear kernel or write_buffer.
                // write_buffer is slow for large arrays? 64KB is fine.
                // Let's just use write_buffer with a zeroed vec.
                let zero_data = vec![0.0f32; grid_size];
                queue.write_buffer(&buffer_a, 0, bytemuck::cast_slice(&zero_data));
                queue.write_buffer(&buffer_b, 0, bytemuck::cast_slice(&zero_data));

                // Run Sim
                let mut current_bind_group = 0; // 0 = A->B
                
                // Batch compute passes? No, we need to update 'time' uniform every step?
                // Updating uniform every step is VERY slow (CPU-GPU sync).
                // Better: Update time in shader using a loop? No, FDTD requires barrier.
                // Compromise: We don't update 'time' every step. We let 'time' be 'step count' * dt.
                // But we can't update uniform inside compute pass.
                // We can assume 'time' is just 'global_id' derived? No.
                // Actually, for a logic gate, we need STABLE output.
                // Let's set parameters once, and let the shader handle time? 
                // Shader uniform `time` is constant for the dispatch?
                // If we dispatch 300 times, we need to update time 300 times? Too slow.
                //
                // Solution: Push Constants? Or just don't use time-varying inputs?
                // Use Constant Inputs (DC). 
                // "Input A is ON" -> `u_next = 1.0`. "Input A is OFF" -> `u_next = 0.0`.
                // Continuous Wave (CW) source. 
                // If we assume CW, we don't need `sin(time)`. We just set source = 1.0.
                // This simplifies everything.
                
                let mut params = SimParams {
                    width: GRID_WIDTH, height: GRID_HEIGHT, dt: DT, dx: DX,
                    input_a_active: *in_a, input_b_active: *in_b, bias_active: 1.0,
                    time: 0.0, 
                };
                queue.write_buffer(&param_buffer, 0, bytemuck::cast_slice(&[params]));

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
                    cpass.set_pipeline(&pipeline);
                    
                    for _ in 0..SIM_STEPS {
                        if current_bind_group == 0 {
                            cpass.set_bind_group(0, &bg_a_to_b, &[]);
                            current_bind_group = 1;
                        } else {
                            cpass.set_bind_group(0, &bg_b_to_a, &[]);
                            current_bind_group = 0;
                        }
                        cpass.dispatch_workgroups(GRID_WIDTH/16, GRID_HEIGHT/16, 1);
                    }
                }
                
                // Copy result to readback
                // Result is in the buffer we LAST wrote to.
                // If current=0, we are ABOUT to write to B. So last write was to A (via bg_b_to_a).
                // Wait. Loop ends. current_bind_group is set for NEXT iteration.
                // If current=0, last was 1. Last pass used bg_b_to_a (wrote to A). So A has data.
                // If current=1, last was 0. Last pass used bg_a_to_b (wrote to B). So B has data.
                let source = if current_bind_group == 0 { &buffer_a } else { &buffer_b };
                encoder.copy_buffer_to_buffer(source, 0, &readback_buffer, 0, grid_bytes);
                
                queue.submit(Some(encoder.finish()));
                
                // Map and Read
                let buffer_slice = readback_buffer.slice(..);
                let (tx, rx) = tokio::sync::oneshot::channel();
                buffer_slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
                device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();
                rx.await.unwrap().unwrap();
                
                {
                    let data = buffer_slice.get_mapped_range();
                    let floats: &[f32] = bytemuck::cast_slice(&data);
                    
                    // Measure Detectors
                    let mut detected_energy = 0.0;
                    for (dx, dy) in DETECTORS {
                        let val = floats[(dy * GRID_WIDTH + dx) as usize];
                        detected_energy += val.abs();
                    }
                    let avg_output = detected_energy / (DETECTORS.len() as f32);
                    
                    // Score
                    // If target is 1.0, we want High Energy (e.g. > 0.5)
                    // If target is 0.0, we want Low Energy (e.g. < 0.1)
                    // Error = (Output - Target)^2
                    
                    // Normalize output? 
                    // FDTD energy can grow > 1.0 due to constructive interference.
                    // Let's clamp it or use a soft target.
                    let clamped_out = avg_output.clamp(0.0, 1.0);
                    let diff = clamped_out - target;
                    total_error += diff * diff;
                }
                readback_buffer.unmap();
            }
            
            let fitness = 1.0 / (1.0 + total_error);
            fitness_scores.push((idx, fitness));
        }

        // Sort by Fitness
        fitness_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let best_fit = fitness_scores[0].1;
        let best_idx = fitness_scores[0].0;
        
        println!("Gen {} Best Fitness: {:.4} (Genome {})", generation_num, best_fit, best_idx);
        // println!("Genome: {:?}", population[best_idx]); // Optional: Print genome

        // Breeding (Elitism)
        let mut new_population = Vec::new();
        
        // Keep Elites
        for i in 0..ELITISM_COUNT {
            if i < fitness_scores.len() {
                new_population.push(population[fitness_scores[i].0].clone());
            }
        }
        
        // Fill rest
        while new_population.len() < POPULATION_SIZE {
            if rng.random_bool(0.3) {
                // Random New
                new_population.push(SdfOp::random(5));
            } else {
                // Mutate from Top 50%
                let parent_idx = fitness_scores[rng.random_range(0..POPULATION_SIZE/2)].0;
                let child = population[parent_idx].mutate(0.1);
                new_population.push(child);
            }
        }
        
        population = new_population;
    }
    
    println!("Evolution Complete.");
}