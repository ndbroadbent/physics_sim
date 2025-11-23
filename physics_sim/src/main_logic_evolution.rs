use physics_sim::shader_gen::{SdfOp, TransformOp};
use wgpu::util::DeviceExt;
use rand::Rng;
use std::fs;
use std::path::Path;

// Evolution Parameters
const POPULATION_SIZE: usize = 20;
const GENERATIONS: usize = 20;
const MUTATION_RATE: f32 = 0.3;

// FDTD Parameters
const GRID_WIDTH: u32 = 128; // Small grid for fast evaluation
const GRID_HEIGHT: u32 = 128;
const SIM_STEPS: u32 = 300; // Enough time for wave to cross
const DT: f32 = 0.5; // Fast time step (stability limit is ~0.7)
const DX: f32 = 1.0;

// Data Structs
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

// Template Parts
const SHADER_TEMPLATE: &str = include_str!("logic.wgsl");

// Helper functions (copied from shader_gen for injection)
const SHADER_HELPERS: &str = r#"// Helper functions
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
"#;

#[tokio::main]
async fn main() {
    env_logger::init();
    println!("Initializing Logic Gate Evolution...");

    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default()).await.unwrap();

    let mut population: Vec<SdfOp> = (0..POPULATION_SIZE).map(|_| SdfOp::random(4)).collect();
    let mut fitness_scores: Vec<f32> = vec![0.0; POPULATION_SIZE];
    let mut best_fitness = -1.0;
    let mut best_genome_idx = 0;

    // Truth Table for NAND (A, B) -> Out
    // Bias is always ON.
    // 0,0 -> 1
    // 0,1 -> 1
    // 1,0 -> 1
    // 1,1 -> 0
    let test_cases = vec![
        (0.0, 0.0, 1.0), // Target High
        (0.0, 1.0, 1.0), // Target High
        (1.0, 0.0, 1.0), // Target High
        (1.0, 1.0, 0.0), // Target Low (Destructive Interference)
    ];

    let grid_size = (GRID_WIDTH * GRID_HEIGHT) as usize;
    
    // Buffers (Reused)
    let buffer_a = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Buffer A"), size: (grid_size * 4) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false
    });
    let buffer_b = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Buffer B"), size: (grid_size * 4) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging"), size: (grid_size * 4) as u64, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false
    });

    let mut rng = rand::rng();

    for generation_num in 0..GENERATIONS {
        println!("\n--- Generation {} ---", generation_num);

        for (idx, genome) in population.iter().enumerate() {
            // 1. Compile Shader
            let geom_code = genome.to_wgsl("p");
            let full_source = SHADER_TEMPLATE
                .replace("// INSERT_GENERATED_CODE_HERE", &format!("    return {};", geom_code))
                .replace("// Helper functions", SHADER_HELPERS);

            let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(full_source.into()),
            });

            // 2. Run Test Cases
            let mut total_error = 0.0;

            for (in_a, in_b, target) in &test_cases {
                // Reset Grid (Zero)
                // Requires clearing buffer... faster to just dispatch a clear kernel or writeZero.
                // For prototype, assume zero init overhead is negligible compared to 300 steps.
                // TODO: Clear buffers.
                
                // Create Pipeline (Expensive to do in inner loop? No, pipeline depends on shader).
                // Bind groups depend on buffers (static).
                // Params depend on test case.
                
                let params = SimParams {
                    width: GRID_WIDTH, height: GRID_HEIGHT, dt: DT, dx: DX,
                    input_a_active: *in_a, input_b_active: *in_b, bias_active: 1.0,
                    time: 0.0,
                };
                let param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None, contents: bytemuck::cast_slice(&[params]), usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                // ... (Bind Group creation) ...
                // ... (Run loop SIM_STEPS) ...
                
                // Read detector pixel (Right side, middle)
                // detector_val = ...
                
                // Error += (detector_val - target).abs();
            }
            
            // Fitness = 1.0 / (1.0 + total_error);
            // fitness_scores[idx] = ...
        }
        
        // Breeding...
    }
}
