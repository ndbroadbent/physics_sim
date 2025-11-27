#[path = "../gate_universal.rs"]
mod gate_universal;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use wgpu::Maintain;
use std::borrow::Cow;
use std::collections::HashSet;

const POPULATION_SIZE: usize = 50000;
const GENOME_SIZE: usize = 64; // 8x8
const LEARNING_RATE: f32 = 0.05;
const GRID_SIZE: usize = 256;
const SIM_STEPS: usize = 20;

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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct IOConfig {
    inputs: [u32; 8],
    outputs: [u32; 8],
}

impl Genome {
    fn new_random(rng: &mut impl Rng) -> Self {
        let mut cells = Vec::with_capacity(GENOME_SIZE);
        for _ in 0..GENOME_SIZE {
            cells.push(Cell {
                op: rng.gen_range(0..17),
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
    op_probs: Vec<Vec<f32>>,
    dir_a_probs: Vec<Vec<f32>>,
    dir_b_probs: Vec<Vec<f32>>,
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
            let op = cell.op as usize;
            if op < 17 {
                for p in &mut self.op_probs[i] { *p *= one_minus; }
                self.op_probs[i][op] += LEARNING_RATE;
                normalize(&mut self.op_probs[i]);
            }
            let da = cell.dir_a as usize;
            if da < 8 {
                for p in &mut self.dir_a_probs[i] { *p *= one_minus; }
                self.dir_a_probs[i][da] += LEARNING_RATE;
                normalize(&mut self.dir_a_probs[i]);
            }
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
    async fn new(io_config: &IOConfig) -> Self {
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
            entries: &
                [
                    wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
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

        let io_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("IO Config"),
            contents: bytemuck::bytes_of(io_config),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let results_size = (POPULATION_SIZE * 2 * 4) as u64;
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
            entries: &
                [
                    wgpu::BindGroupEntry { binding: 0, resource: self.genomes_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: self.io_buffer.as_entire_binding() }, // Use io_buffer
                    wgpu::BindGroupEntry { binding: 2, resource: self.results_buffer.as_entire_binding() },
                ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(POPULATION_SIZE as u32, 1, 1);
        }

        encoder.copy_buffer_to_buffer(&self.results_buffer, 0, &self.staging_buffer, 0, (POPULATION_SIZE * 2 * 4) as u64);
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = self.staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
        self.device.poll(Maintain::Wait);
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
    let mut indices: Vec<u32> = (0..256).collect();
    for i in (1..indices.len()).rev() {
        let j = rng.gen_range(0..=i);
        indices.swap(i, j);
    }
    let mut inputs = [0u32; 8];
    let mut outputs = [0u32; 8];
    for i in 0..8 { inputs[i] = indices[i]; }
    for i in 0..8 { outputs[i] = indices[8+i]; }
    
    let io_config = IOConfig { inputs, outputs };
    
    println!("Inputs: {:?}", inputs);
    println!("Outputs: {:?}", outputs);

    let gpu = pollster::block_on(GpuContext::new(&io_config));
    let mut prob_matrix = ProbMatrix::new();
    
    let mut gen = 0;
    let mut best_match = 0;

    loop {
        if gen >= 200 { break; }
        
        let population: Vec<Genome> = (0..POPULATION_SIZE).map(|_| prob_matrix.sample(&mut rng)).collect();
        
        let results_packed = gpu.evaluate_batch(&population);
        
        let mut next_gen = Vec::with_capacity(POPULATION_SIZE);
        let mut best_idx = 0;
        let mut max_score = 0;
        
        for (i, g) in population.iter().enumerate() {
            let r_offset = i * 2;
            let r_u32s = &results_packed[r_offset..r_offset+2];
            
            // Trace Active
            let active_map = trace_active(g, &io_config);
            let mut active_cells = 0;
            let mut matches = 0;
            
            for cell_idx in 0..GENOME_SIZE {
                // Map genome cell to grid index
                let org_x = cell_idx % 8;
                let org_y = cell_idx / 8;
                let grid_idx = (4 + org_y) * 16 + (4 + org_x);
                
                if !active_map[grid_idx] { continue; }
                
                active_cells += 1;
                
                let output_op = if cell_idx < 8 {
                    (r_u32s[0] >> (cell_idx * 4)) & 0xF
                } else {
                    (r_u32s[1] >> ((cell_idx - 8) * 4)) & 0xF
                };
                
                let target_op = g.cells[cell_idx].op;
                let effective_target = if target_op == 16 { 0 } else { target_op };
                
                if output_op == effective_target { matches += 1; }
            }
            
            if active_cells < 10 { continue; } 
            
            let score = matches; 
            
            if score == active_cells {
                if active_cells > best_match {
                    best_match = active_cells;
                    best_idx = i;
                    println!("New Best: Gen {} | Active Quine: {} / {} cells", gen, active_cells, active_cells);
                    
                    if active_cells >= 20 { 
                        println!("GRID QUINE SOLVED!");
                        print_grid(&population[best_idx]);
                        return;
                    }
                }
            }
            
            if score > max_score {
                max_score = score;
                best_idx = i;
            }
        }
        
        // Update matrix with best
        if max_score > 0 {
             prob_matrix.update(&population[best_idx]);
        }
        
        if gen % 10 == 0 {
            println!("Gen {} | Best Active Match: {} | Entropy: {:.2}", gen, max_score, prob_matrix.entropy());
        }
        
        gen += 1;
    }
}

fn trace_active(genome: &Genome, io_config: &IOConfig) -> Vec<bool> {
    let mut active_at_step = vec![HashSet::new(); SIM_STEPS + 1];
    let mut globally_active = vec![false; GRID_SIZE];
    
    // Seed with Output Ports
    for &out_idx in &io_config.outputs {
        active_at_step[SIM_STEPS].insert(out_idx as usize);
    }
    
    // Propagate
    for t in (1..=SIM_STEPS).rev() {
        // Clone to avoid double borrow
        let current_active: Vec<usize> = active_at_step[t].iter().cloned().collect();
        for &idx in &current_active {
            globally_active[idx] = true;
            
            // Map to organism
            let x = idx % 16;
            let y = idx / 16;
            
            let mut op = 16;
            let mut dir_a = 0;
            let mut dir_b = 0;
            
            if x >= 4 && x < 12 && y >= 4 && y < 12 {
                let org_x = x - 4;
                let org_y = y - 4;
                let org_idx = org_y * 8 + org_x;
                let cell = &genome.cells[org_idx];
                op = cell.op;
                dir_a = cell.dir_a;
                dir_b = cell.dir_b;
            }
            
            // Check Input
            let mut is_input = false;
            for &inp in &io_config.inputs { if inp as usize == idx { is_input = true; break; } }
            if is_input { continue; }
            
            if op < 16 {
                let na = get_neighbor_idx(idx as i32, dir_a);
                let nb = get_neighbor_idx(idx as i32, dir_b);
                active_at_step[t-1].insert(na);
                active_at_step[t-1].insert(nb);
            }
        }
    }
    globally_active
}

fn get_neighbor_idx(idx: i32, dir: u32) -> usize {
    let x = idx % 16;
    let y = idx / 16;
    let mut dx = 0; let mut dy = 0;
    match dir {
        0 => { dx = 0; dy = -1; }
        1 => { dx = 1; dy = -1; }
        2 => { dx = 1; dy = 0; }
        3 => { dx = 1; dy = 1; }
        4 => { dx = 0; dy = 1; }
        5 => { dx = -1; dy = 1; }
        6 => { dx = -1; dy = 0; }
        7 => { dx = -1; dy = -1; }
        _ => {}
    }
    let nx = (x + dx + 16) % 16;
    let ny = (y + dy + 16) % 16;
    (ny * 16 + nx) as usize
}

fn print_grid(genome: &Genome) {
    println!("\n--- Organism Structure (8x8) ---");
    println!("Format: [Op A B]");
    println!("Op: 0-F (Logic), X (Void)");
    println!("Dirs: 0:N 1:NE 2:E 3:SE 4:S 5:SW 6:W 7:NW");
    println!("--------------------------------");
    
    let dir_chars = ['↑', '↗', '→', '↘', '↓', '↙', '←', '↖'];

    for y in 0..8 {
        for x in 0..8 {
            let idx = y * 8 + x;
            let cell = &genome.cells[idx];
            let op_char = if cell.op < 16 { format!("{:X}", cell.op) } else { "X".to_string() };
            
            let da_char = if cell.dir_a < 8 { dir_chars[cell.dir_a as usize] } else { '?' };
            let db_char = if cell.dir_b < 8 { dir_chars[cell.dir_b as usize] } else { '?' };

            print!("[{} {} {}] ", op_char, da_char, db_char);
        }
        println!();
    }
    println!("--------------------------------\n");
}