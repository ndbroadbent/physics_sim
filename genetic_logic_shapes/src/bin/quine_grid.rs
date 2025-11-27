#[path = "../gate_universal.rs"]
mod gate_universal;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use wgpu::Maintain; // Used in GpuContext::evaluate_batch
use std::borrow::Cow; // Used in GpuContext::new for shader source

const POPULATION_SIZE: usize = 10000;
const GENOME_SIZE: usize = 64; // 8x8 Organism size
const LEARNING_RATE: f32 = 0.05;

// Constants from shader (used for logic in main)
const SHADER_ORG_DIM: u32 = 8;   // The 8x8 organism
const SHADER_SIM_STEPS: u32 = 20; 
const SHADER_TEST_CASES: u32 = 64; // Number of cells to query (0..63)

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Cell {
    op: u32,
    dir_a: u32,
    dir_b: u32,
    _pad: u32,
}

#[derive(Clone)]
struct Genome {
    cells: Vec<Cell>,
}

impl Genome {
    fn new_random(rng: &mut impl Rng) -> Self {
        let mut cells = Vec::with_capacity(GENOME_SIZE);
        for _ in 0..GENOME_SIZE {
            cells.push(Cell {
                op: rng.gen_range(0..17), // 0-15 Logic, 16 Void
                dir_a: rng.gen_range(0..8),
                dir_b: rng.gen_range(0..8),
                _pad: 0,
            });
        }
        Genome { cells }
    }
    
    fn mutate(&mut self, rate: f64, rng: &mut impl Rng) {
        for cell in &mut self.cells {
            if rng.gen_bool(rate) {
                match rng.gen_range(0..3) {
                    0 => cell.op = rng.gen_range(0..17),
                    1 => cell.dir_a = rng.gen_range(0..8),
                    2 => cell.dir_b = rng.gen_range(0..8),
                    _ => {}
                }
            }
        }
    }
}

struct ProbMatrix {
    op_probs: Vec<Vec<f32>>, // [GENOME_SIZE][17]
    dir_a_probs: Vec<Vec<f32>>, // [GENOME_SIZE][8]
    dir_b_probs: Vec<Vec<f32>>, // [GENOME_SIZE][8]
}

impl ProbMatrix {
    fn new() -> Self {
        let op_uniform = 1.0 / 17.0;
        let dir_uniform = 1.0 / 8.0;
        ProbMatrix {
            op_probs: vec![vec![op_uniform; 17]; GENOME_SIZE], 
            dir_a_probs: vec![vec![dir_uniform; 8]; GENOME_SIZE],
            dir_b_probs: vec![vec![dir_uniform; 8]; GENOME_SIZE],
        }
    }

    fn sample(&self, rng: &mut impl Rng) -> Genome {
        let mut cells = Vec::with_capacity(GENOME_SIZE);
        for i in 0..GENOME_SIZE { 
            let op = sample_discrete(&self.op_probs[i], rng);
            let dir_a = sample_discrete(&self.dir_a_probs[i], rng);
            let dir_b = sample_discrete(&self.dir_b_probs[i], rng);
            cells.push(Cell { op: op as u32, dir_a: dir_a as u32, dir_b: dir_b as u32, _pad: 0 });
        }
        Genome { cells }
    }

    fn update(&mut self, genome: &Genome) { 
        let one_minus = 1.0 - LEARNING_RATE;
        for (i, cell) in genome.cells.iter().enumerate() { 
            // Op
            let op = cell.op as usize;
            if op < 17 {
                for p in &mut self.op_probs[i] { *p *= one_minus; }
                self.op_probs[i][op] += LEARNING_RATE;
                normalize(&mut self.op_probs[i]);
            }
            
            // Dir A
            let da = cell.dir_a as usize;
            if da < 8 {
                for p in &mut self.dir_a_probs[i] { *p *= one_minus; }
                self.dir_a_probs[i][da] += LEARNING_RATE;
                normalize(&mut self.dir_a_probs[i]);
            }

            // Dir B
            let db = cell.dir_b as usize;
            if db < 8 {
                for p in &mut self.dir_b_probs[i] { *p *= one_minus; }
                self.dir_b_probs[i][db] += LEARNING_RATE;
                normalize(&mut self.dir_b_probs[i]);
            }
        }
    }
    
    fn entropy(&self) -> f32 {
        let mut e = 0.0;
        for row in &self.op_probs { e += calc_entropy(row); }
        e
    }
    
    fn inject_noise(&mut self, amount: f32) {
        let uniform_op = 1.0 / 17.0;
        let uniform_dir = 1.0 / 8.0;
        
        for row in &mut self.op_probs {
            for p in row.iter_mut() {
                *p = *p * (1.0 - amount) + uniform_op * amount;
            }
            normalize(row);
        }
        for row in &mut self.dir_a_probs {
            for p in row.iter_mut() {
                *p = *p * (1.0 - amount) + uniform_dir * amount;
            }
            normalize(row);
        }
        for row in &mut self.dir_b_probs { 
            for p in row.iter_mut() {
                *p = *p * (1.0 - amount) + uniform_dir * amount;
            }
            normalize(row);
        }
    }
}

fn sample_discrete(probs: &[f32], rng: &mut impl Rng) -> usize {
    let r = rng.gen::<f32>();
    let mut sum = 0.0;
    for (i, &p) in probs.iter().enumerate() {
        sum += p;
        if r < sum { return i; }
    }
    probs.len() - 1
}

fn normalize(probs: &mut Vec<f32>) {
    let sum: f32 = probs.iter().sum();
    if sum > 0.0 {
        for p in probs.iter_mut() { *p /= sum; }
    }
}

fn calc_entropy(probs: &[f32]) -> f32 {
    let mut h = 0.0;
    for &p in probs {
        if p > 0.0 { h -= p * p.log2(); }
    }
    h
}


struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    genomes_buffer: wgpu::Buffer,
    results_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
}

impl GpuContext {
    async fn new() -> Self {
        let instance = wgpu::Instance::default();
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ).await.unwrap();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Grid Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../quine_grid_evaluator.wgsl"))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let genomes_size = (POPULATION_SIZE * GENOME_SIZE * std::mem::size_of::<Cell>()) as u64;
        let genomes_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Genomes"), size: genomes_size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

        let results_size = (POPULATION_SIZE * 2 * 4) as u64; // 2 u32s per genome
        let results_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Results"), size: results_size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Staging"), size: results_size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

        GpuContext { device, queue, pipeline, bind_group_layout, genomes_buffer, results_buffer, staging_buffer }
    }

    fn evaluate_batch(&self, genomes: &[Genome]) -> Vec<u32> {
        let mut raw_genomes = Vec::with_capacity(POPULATION_SIZE * GENOME_SIZE);
        for g in genomes {
            raw_genomes.extend_from_slice(&g.cells);
        }
        
        self.queue.write_buffer(&self.genomes_buffer, 0, bytemuck::cast_slice(&raw_genomes));
        
        let zeros = vec![0u8; POPULATION_SIZE * 2 * 4];
        self.queue.write_buffer(&self.results_buffer, 0, &zeros);

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.genomes_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: self.results_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            // 1 Workgroup per Genome
            cpass.dispatch_workgroups(POPULATION_SIZE as u32, 1, 1);
        }

        encoder.copy_buffer_to_buffer(&self.results_buffer, 0, &self.staging_buffer, 0, (POPULATION_SIZE * 2 * 4) as u64);
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = self.staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        self.staging_buffer.unmap();

        result
    }
}

fn main() {
    let mut rng = ChaCha8Rng::seed_from_u64(123);
    println!("Initializing Grid Quine Search (8x8)...");
    let gpu = pollster::block_on(GpuContext::new());
    
    let mut prob_matrix = ProbMatrix::new();
    
    let mut population: Vec<Genome> = (0..POPULATION_SIZE)
        .map(|_| prob_matrix.sample(&mut rng))
        .collect();
    
    let mut gen = 0;
    let mut best_match = 0;

    loop {
        if gen >= 200 { break; } // Max 200 generations
        
        let results_packed = gpu.evaluate_batch(&population);
        
        let mut next_gen = Vec::with_capacity(POPULATION_SIZE);
        let mut best_idx = 0;
        let mut max_score = 0;
        
        for (i, g) in population.iter().enumerate() {
            let r0 = results_packed[i * 2];
            let r1 = results_packed[i * 2 + 1];
            
            let mut score = 0;
            for cell_idx in 0..GENOME_SIZE {
                let output_op = if cell_idx < 8 {
                    (r0 >> (cell_idx * 4)) & 0xF
                } else {
                    (r1 >> ((cell_idx - 8) * 4)) & 0xF
                };
                
                let target_op = g.cells[cell_idx].op;
                
                if target_op == 16 { // If VOID, output 0?
                    if output_op == 0 { score += 1; } // Treat 0 output as VOID match?
                } else {
                    if output_op == target_op { score += 1; }
                }
            }
            
            if score > max_score {
                max_score = score;
                best_idx = i;
            }
        }
        
        if max_score > best_match {
            best_match = max_score;
            println!("New Best: Gen {} | Score {} / {}", gen, best_match, GENOME_SIZE);
            if best_match == GENOME_SIZE {
                println!("GRID QUINE SOLVED!");
                print_grid(&population[best_idx]);
                return;
            }
        }
        
        if gen % 10 == 0 {
            println!("Gen {} | Best: {} / {} | Entropy: {:.2}", gen, max_score, GENOME_SIZE, prob_matrix.entropy());
        }
        
        // Elitism + Mutation
        let best_g = population[best_idx].clone();
        next_gen.push(best_g.clone());
        for _ in 1..POPULATION_SIZE {
            let mut child = best_g.clone();
            child.mutate(0.05, &mut rng);
            next_gen.push(child);
        }
        
        population = next_gen;
        gen += 1;
    }
}

fn print_grid(genome: &Genome) {
    println!("\n--- Organism Structure (8x8) ---");
    println!("Format: [Op A B]");
    println!("Op: 0-F (Logic), X (Void)");
    println!("Dirs: 0:N 1:NE 2:E 3:SE 4:S 5:SW 6:W 7:NW");
    println!("--------------------------------");
    
    // Direction chars for visual clarity
    let dir_chars = ['↑', '↗', '→', '↘', '↓', '↙', '←', '↖'];

    for y in 0..8 { // Changed from 0..4
        for x in 0..8 { // Changed from 0..4
            let idx = y * 8 + x; // Changed from y * 4 + x
            let cell = &genome.cells[idx];
            let op_char = if cell.op < 16 { format!("{:X}", cell.op) } else { "X".to_string() };
            
            // Check bounds before using as index
            let da_char = if cell.dir_a < 8 { dir_chars[cell.dir_a as usize] } else { '?' };
            let db_char = if cell.dir_b < 8 { dir_chars[cell.dir_b as usize] } else { '?' };

            print!("[{} {} {}] ", op_char, da_char, db_char);
        }
        println!();
    }
    println!("--------------------------------\n");
}