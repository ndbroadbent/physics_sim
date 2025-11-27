//! 16x16 Quine Evolution
//!
//! Evolves organisms that can describe their own structure.
//! Each cell has: op (4b), dir_a (3b), dir_b (3b), is_input (1b), is_output (1b)
//! The organism must output its own 12-bit cell descriptors when queried.
//!
//! Key insight: Only ACTIVE cells (connected to outputs) need to be correct.
//! "Junk DNA" that isn't connected to the output path doesn't matter.
//! Outputs are constrained to the south edge for the "painting" metaphor.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::borrow::Cow;
use std::collections::HashSet;
use wgpu::Maintain;

const POPULATION_SIZE: usize = 8_000; // Limited by GPU buffer size
const GENOME_SIZE: usize = 256; // 16x16
const GRID_DIM: usize = 16;
const LEARNING_RATE: f32 = 0.05;
const SIM_STEPS: usize = 30;
const MIN_SOUTH_OUTPUTS: usize = 12; // Minimum outputs required (for 12-bit descriptor)

// Results: 256 cells * 12 bits = 3072 bits = 96 u32s per genome
const RESULTS_U32S_PER_GENOME: usize = 96;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Cell {
    op: u32,    // 0-15 logic gates, 16 = VOID
    dir_a: u32, // 0-7 direction
    dir_b: u32, // 0-7 direction
    flags: u32, // bit 0: is_input, bit 1: is_output
}

impl Cell {
    fn is_input(&self) -> bool {
        self.flags & 1 != 0
    }

    fn is_output(&self) -> bool {
        self.flags & 2 != 0
    }

    /// Encode cell as 12-bit descriptor: [op:4][dir_a:3][dir_b:3][is_input:1][is_output:1]
    fn encode(&self) -> u32 {
        let op = self.op.min(15); // Clamp to 4 bits (VOID becomes 0 in output)
        let dir_a = self.dir_a & 7;
        let dir_b = self.dir_b & 7;
        let is_in = if self.is_input() { 1 } else { 0 };
        let is_out = if self.is_output() { 1 } else { 0 };
        (op << 8) | (dir_a << 5) | (dir_b << 2) | (is_in << 1) | is_out
    }
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
                op: rng.gen_range(0..17),    // 0-15 logic, 16 = void
                dir_a: rng.gen_range(0..8),
                dir_b: rng.gen_range(0..8),
                flags: rng.gen_range(0..4),  // 2 bits: is_input, is_output
            });
        }
        Genome { cells }
    }
}

/// Probability matrix for EDA (Estimation of Distribution Algorithm)
struct ProbMatrix {
    op_probs: Vec<Vec<f32>>,      // [256][17]
    dir_a_probs: Vec<Vec<f32>>,   // [256][8]
    dir_b_probs: Vec<Vec<f32>>,   // [256][8]
    flags_probs: Vec<Vec<f32>>,   // [256][4] - 2 bits = 4 combinations
}

impl ProbMatrix {
    fn new() -> Self {
        let op_uniform = 1.0 / 17.0;
        let dir_uniform = 1.0 / 8.0;
        let flags_uniform = 1.0 / 4.0;
        ProbMatrix {
            op_probs: vec![vec![op_uniform; 17]; GENOME_SIZE],
            dir_a_probs: vec![vec![dir_uniform; 8]; GENOME_SIZE],
            dir_b_probs: vec![vec![dir_uniform; 8]; GENOME_SIZE],
            flags_probs: vec![vec![flags_uniform; 4]; GENOME_SIZE],
        }
    }

    fn sample(&self, rng: &mut impl Rng) -> Genome {
        let mut cells = Vec::with_capacity(GENOME_SIZE);
        for i in 0..GENOME_SIZE {
            let op = sample_discrete(&self.op_probs[i], rng) as u32;
            let dir_a = sample_discrete(&self.dir_a_probs[i], rng) as u32;
            let dir_b = sample_discrete(&self.dir_b_probs[i], rng) as u32;
            let flags = sample_discrete(&self.flags_probs[i], rng) as u32;
            cells.push(Cell { op, dir_a, dir_b, flags });
        }
        Genome { cells }
    }

    fn update(&mut self, genome: &Genome) {
        let one_minus = 1.0 - LEARNING_RATE;
        for (i, cell) in genome.cells.iter().enumerate() {
            // Update op probabilities
            let op = (cell.op as usize).min(16);
            for p in &mut self.op_probs[i] {
                *p *= one_minus;
            }
            self.op_probs[i][op] += LEARNING_RATE;
            normalize(&mut self.op_probs[i]);

            // Update dir_a probabilities
            let da = (cell.dir_a as usize) & 7;
            for p in &mut self.dir_a_probs[i] {
                *p *= one_minus;
            }
            self.dir_a_probs[i][da] += LEARNING_RATE;
            normalize(&mut self.dir_a_probs[i]);

            // Update dir_b probabilities
            let db = (cell.dir_b as usize) & 7;
            for p in &mut self.dir_b_probs[i] {
                *p *= one_minus;
            }
            self.dir_b_probs[i][db] += LEARNING_RATE;
            normalize(&mut self.dir_b_probs[i]);

            // Update flags probabilities
            let f = (cell.flags as usize) & 3;
            for p in &mut self.flags_probs[i] {
                *p *= one_minus;
            }
            self.flags_probs[i][f] += LEARNING_RATE;
            normalize(&mut self.flags_probs[i]);
        }
    }

    fn entropy(&self) -> f32 {
        let mut e = 0.0;
        for row in &self.op_probs {
            e += calc_entropy(row);
        }
        e
    }
}

fn sample_discrete(probs: &[f32], rng: &mut impl Rng) -> usize {
    let r = rng.gen::<f32>();
    let mut sum = 0.0;
    for (i, &p) in probs.iter().enumerate() {
        sum += p;
        if r < sum {
            return i;
        }
    }
    probs.len() - 1
}

fn normalize(probs: &mut [f32]) {
    let sum: f32 = probs.iter().sum();
    if sum > 0.0 {
        for p in probs.iter_mut() {
            *p /= sum;
        }
    }
}

fn calc_entropy(probs: &[f32]) -> f32 {
    let mut h = 0.0;
    for &p in probs {
        if p > 0.0 {
            h -= p * p.log2();
        }
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
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Quine 16x16 Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../quine_16x16_evaluator.wgsl"
            ))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
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
            label: Some("Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        // Genome buffer: 256 cells * 16 bytes per cell * 50K organisms
        let genomes_size = (POPULATION_SIZE * GENOME_SIZE * std::mem::size_of::<Cell>()) as u64;
        let genomes_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Genomes"),
            size: genomes_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Results buffer: 96 u32s per genome * 50K organisms
        let results_size = (POPULATION_SIZE * RESULTS_U32S_PER_GENOME * 4) as u64;
        let results_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Results"),
            size: results_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: results_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        GpuContext {
            device,
            queue,
            pipeline,
            bind_group_layout,
            genomes_buffer,
            results_buffer,
            staging_buffer,
        }
    }

    fn evaluate_batch(&self, genomes: &[Genome]) -> Vec<u32> {
        let mut raw_genomes = Vec::with_capacity(POPULATION_SIZE * GENOME_SIZE);
        for g in genomes {
            raw_genomes.extend_from_slice(&g.cells);
        }

        self.queue
            .write_buffer(&self.genomes_buffer, 0, bytemuck::cast_slice(&raw_genomes));

        // Zero out results
        let zeros = vec![0u8; POPULATION_SIZE * RESULTS_U32S_PER_GENOME * 4];
        self.queue.write_buffer(&self.results_buffer, 0, &zeros);

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.genomes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.results_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(POPULATION_SIZE as u32, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &self.results_buffer,
            0,
            &self.staging_buffer,
            0,
            (POPULATION_SIZE * RESULTS_U32S_PER_GENOME * 4) as u64,
        );
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

/// Calculate quine fitness for a genome given GPU results
/// Only checks ACTIVE cells (connected to outputs) - junk DNA is ignored
fn quine_fitness(genome: &Genome, results: &[u32], active_cells: &HashSet<usize>) -> (usize, usize, usize) {
    let mut matches = 0;
    let mut total_bits_correct = 0;
    let active_count = active_cells.len();

    for &cell_idx in active_cells {
        let expected = genome.cells[cell_idx].encode();

        // Extract 12-bit result from packed u32s
        // Each cell produces 12 bits, packed sequentially
        let bit_offset = cell_idx * 12;
        let word_idx = bit_offset / 32;
        let bit_in_word = bit_offset % 32;

        let actual = if bit_in_word + 12 <= 32 {
            // Fits in one word
            (results[word_idx] >> bit_in_word) & 0xFFF
        } else {
            // Spans two words
            let lo_bits = 32 - bit_in_word;
            let hi_bits = 12 - lo_bits;
            let lo = results[word_idx] >> bit_in_word;
            let hi = results[word_idx + 1] & ((1 << hi_bits) - 1);
            lo | (hi << lo_bits)
        };

        if expected == actual {
            matches += 1;
            total_bits_correct += 12;
        } else {
            // Count matching bits for partial credit
            let xor = expected ^ actual;
            total_bits_correct += 12 - xor.count_ones() as usize;
        }
    }

    (matches, total_bits_correct, active_count)
}

/// Count input and output cells in a genome
fn count_io(genome: &Genome) -> (usize, usize) {
    let inputs = genome.cells.iter().filter(|c| c.is_input()).count();
    let outputs = genome.cells.iter().filter(|c| c.is_output()).count();
    (inputs, outputs)
}

/// Get neighbor index given direction (0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW)
fn get_neighbor_idx(idx: usize, dir: u32) -> usize {
    let x = (idx % GRID_DIM) as i32;
    let y = (idx / GRID_DIM) as i32;
    let (dx, dy) = match dir {
        0 => (0, -1),  // N
        1 => (1, -1),  // NE
        2 => (1, 0),   // E
        3 => (1, 1),   // SE
        4 => (0, 1),   // S
        5 => (-1, 1),  // SW
        6 => (-1, 0),  // W
        7 => (-1, -1), // NW
        _ => (0, 0),
    };
    let nx = (x + dx).rem_euclid(GRID_DIM as i32) as usize;
    let ny = (y + dy).rem_euclid(GRID_DIM as i32) as usize;
    ny * GRID_DIM + nx
}

/// Trace active cells backward from outputs to inputs
/// Returns a set of cell indices that are "live" (contribute to output)
fn trace_active(genome: &Genome) -> HashSet<usize> {
    let mut active_at_step: Vec<HashSet<usize>> = vec![HashSet::new(); SIM_STEPS + 1];
    let mut globally_active = HashSet::new();

    // Seed with output cells (south edge: y=15, x=0..15)
    for x in 0..GRID_DIM {
        let idx = 15 * GRID_DIM + x; // South edge
        if genome.cells[idx].is_output() {
            active_at_step[SIM_STEPS].insert(idx);
        }
    }

    // Propagate backward through time
    for t in (1..=SIM_STEPS).rev() {
        let current_active: Vec<usize> = active_at_step[t].iter().cloned().collect();
        for &idx in &current_active {
            globally_active.insert(idx);

            let cell = &genome.cells[idx];

            // Input cells are sources - don't trace further
            if cell.is_input() {
                continue;
            }

            // If this cell computes (op < 16), trace to its inputs
            if cell.op < 16 {
                let na = get_neighbor_idx(idx, cell.dir_a);
                let nb = get_neighbor_idx(idx, cell.dir_b);
                active_at_step[t - 1].insert(na);
                active_at_step[t - 1].insert(nb);
            }
        }
    }

    globally_active
}

/// Count how many south edge cells are outputs
fn count_south_outputs(genome: &Genome) -> usize {
    (0..GRID_DIM)
        .filter(|&x| genome.cells[15 * GRID_DIM + x].is_output())
        .count()
}

fn print_genome(genome: &Genome) {
    println!("\n--- Organism Structure (16x16) ---");
    println!("I=input O=output B=both _=none");
    println!("Format: [Op IO]");

    let dir_chars = ['↑', '↗', '→', '↘', '↓', '↙', '←', '↖'];

    for y in 0..GRID_DIM {
        for x in 0..GRID_DIM {
            let idx = y * GRID_DIM + x;
            let cell = &genome.cells[idx];
            let op_char = if cell.op < 16 {
                format!("{:X}", cell.op)
            } else {
                ".".to_string()
            };

            let io_char = match (cell.is_input(), cell.is_output()) {
                (true, true) => 'B',
                (true, false) => 'I',
                (false, true) => 'O',
                (false, false) => '_',
            };

            print!("{}{} ", op_char, io_char);
        }
        println!();
    }
    println!("----------------------------------\n");
}

fn main() {
    println!("=== 16x16 Quine Evolution (Active Cells Only) ===");
    println!("Genome: 256 cells, 12 bits each (op + dirs + flags)");
    println!("Requires minimum {} outputs on south edge", MIN_SOUTH_OUTPUTS);
    println!("Only ACTIVE cells (connected to outputs) are scored");
    println!("Junk DNA is ignored - only the live circuit matters\n");

    let mut rng = ChaCha8Rng::seed_from_u64(42);

    let gpu = pollster::block_on(GpuContext::new());
    let mut prob_matrix = ProbMatrix::new();

    let mut best_ever_matches = 0;
    let mut best_ever_active = 0;
    let mut best_ever_ratio = 0.0f32;

    for gen in 0..1000 {
        // Sample population from probability matrix
        let population: Vec<Genome> = (0..POPULATION_SIZE)
            .map(|_| prob_matrix.sample(&mut rng))
            .collect();

        // Evaluate on GPU
        let all_results = gpu.evaluate_batch(&population);

        // Find best genome
        let mut best_idx = 0;
        let mut best_matches = 0;
        let mut best_active = 0;
        let mut best_ratio = 0.0f32;

        for (i, g) in population.iter().enumerate() {
            let result_offset = i * RESULTS_U32S_PER_GENOME;
            let results = &all_results[result_offset..result_offset + RESULTS_U32S_PER_GENOME];

            // Count south edge outputs - must have at least MIN_SOUTH_OUTPUTS
            let south_outputs = count_south_outputs(g);
            if south_outputs < MIN_SOUTH_OUTPUTS {
                continue; // Skip organisms with too few outputs
            }

            // Trace active cells from south edge outputs
            let active_cells = trace_active(g);

            // Skip if no active cells
            if active_cells.is_empty() {
                continue;
            }

            let (matches, bits, active_count) = quine_fitness(g, results, &active_cells);

            // NEW FITNESS: Prefer more TOTAL bits correct (encourages larger circuits)
            // Tie-break with more active cells (larger circuits can learn more)
            if bits > best_matches || (bits == best_matches && active_count > best_active) {
                best_matches = bits;
                best_active = active_count;
                best_ratio = matches as f32 / active_count as f32;
                best_idx = i;
            }
        }

        // Update probability matrix with best genome
        if best_active > 0 {
            prob_matrix.update(&population[best_idx]);
        }

        // Track best ever (based on total bits correct)
        if best_matches > best_ever_matches
            || (best_matches == best_ever_matches && best_active > best_ever_active)
        {
            best_ever_matches = best_matches;
            best_ever_active = best_active;
            best_ever_ratio = best_ratio;

            let (inputs, _outputs) = count_io(&population[best_idx]);
            let south_outputs = count_south_outputs(&population[best_idx]);
            let total_bits = best_active * 12; // Total bits to match
            println!(
                "NEW BEST Gen {} | {}/{} bits ({:.1}%) | Active: {} | South: {} | In: {}",
                gen, best_matches, total_bits, 100.0 * best_matches as f32 / total_bits as f32,
                best_active, south_outputs, inputs
            );

            if best_ratio >= 0.95 && best_active >= 20 {
                println!("\n*** QUINE NEARLY SOLVED! ***");
                print_genome(&population[best_idx]);
            }

            if best_ratio == 1.0 && best_active >= 30 {
                println!("\n*** PERFECT QUINE! ***");
                print_genome(&population[best_idx]);
                return;
            }
        }

        if gen % 20 == 0 {
            let (inputs, _outputs) = count_io(&population[best_idx]);
            let south_outputs = count_south_outputs(&population[best_idx]);
            let total_bits = best_active * 12;
            println!(
                "Gen {:4} | {}/{} bits ({:.1}%) | Active: {} | South: {} | In: {} | Entropy: {:.1}",
                gen, best_matches, total_bits, 100.0 * best_matches as f32 / total_bits.max(1) as f32,
                best_active, south_outputs, inputs, prob_matrix.entropy()
            );
        }
    }

    let total_bits = best_ever_active * 12;
    println!("\nEvolution complete. Best: {}/{} bits ({:.1}%)",
             best_ever_matches, total_bits, 100.0 * best_ever_matches as f32 / total_bits.max(1) as f32);
}
