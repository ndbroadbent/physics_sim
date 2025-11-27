//! Evolve XOR Gate
//!
//! The simplest evolution test: learn to compute XOR.
//! Input: 2 bits (A, B)
//! Output: 1 bit (A XOR B)
//!
//! Uses a tiny 4x4 grid (16 cells).
//! Fitness = number of correct outputs across 4 test cases (max 4).

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::borrow::Cow;
use wgpu::Maintain;

const POPULATION_SIZE: usize = 10_000;
const GENOME_SIZE: usize = 16; // 4x4
const GRID_DIM: usize = 4;
const LEARNING_RATE: f32 = 0.1;
const EXPLORATION_RATE: f32 = 0.1;
const MAX_FITNESS: usize = 4; // 4 test cases

const NUM_OPS: usize = 18; // 0-15 logic, 16 VOID, 17 JUMP

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Cell {
    op: u32,
    dir_a: u32,
    dir_b: u32,
    flags: u32,
}

#[derive(Clone)]
struct Genome {
    cells: Vec<Cell>,
}

struct ProbMatrix {
    op_probs: Vec<Vec<f32>>,
    dir_a_probs: Vec<Vec<f32>>,
    dir_b_probs: Vec<Vec<f32>>,
}

impl ProbMatrix {
    fn new() -> Self {
        let op_uniform = 1.0 / NUM_OPS as f32;
        let dir_uniform = 1.0 / 8.0;
        ProbMatrix {
            op_probs: vec![vec![op_uniform; NUM_OPS]; GENOME_SIZE],
            dir_a_probs: vec![vec![dir_uniform; 8]; GENOME_SIZE],
            dir_b_probs: vec![vec![dir_uniform; 8]; GENOME_SIZE],
        }
    }

    fn sample(&self, rng: &mut impl Rng) -> Genome {
        let mut cells = Vec::with_capacity(GENOME_SIZE);
        for i in 0..GENOME_SIZE {
            let (op, dir_a, dir_b) = if rng.gen::<f32>() < EXPLORATION_RATE {
                (
                    rng.gen_range(0..NUM_OPS as u32),
                    rng.gen_range(0..8u32),
                    rng.gen_range(0..8u32),
                )
            } else {
                (
                    sample_discrete(&self.op_probs[i], rng) as u32,
                    sample_discrete(&self.dir_a_probs[i], rng) as u32,
                    sample_discrete(&self.dir_b_probs[i], rng) as u32,
                )
            };
            cells.push(Cell { op, dir_a, dir_b, flags: 0 });
        }
        Genome { cells }
    }

    fn update(&mut self, genome: &Genome) {
        let one_minus = 1.0 - LEARNING_RATE;
        for (i, cell) in genome.cells.iter().enumerate() {
            let op = (cell.op as usize).min(NUM_OPS - 1);
            for p in &mut self.op_probs[i] {
                *p *= one_minus;
            }
            self.op_probs[i][op] += LEARNING_RATE;
            normalize(&mut self.op_probs[i]);

            let da = (cell.dir_a as usize) & 7;
            for p in &mut self.dir_a_probs[i] {
                *p *= one_minus;
            }
            self.dir_a_probs[i][da] += LEARNING_RATE;
            normalize(&mut self.dir_a_probs[i]);

            let db = (cell.dir_b as usize) & 7;
            for p in &mut self.dir_b_probs[i] {
                *p *= one_minus;
            }
            self.dir_b_probs[i][db] += LEARNING_RATE;
            normalize(&mut self.dir_b_probs[i]);
        }
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
            label: Some("XOR Evaluator Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../xor_evaluator.wgsl"
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

        let genomes_size = (POPULATION_SIZE * GENOME_SIZE * std::mem::size_of::<Cell>()) as u64;
        let genomes_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Genomes"),
            size: genomes_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let results_size = (POPULATION_SIZE * 4) as u64;
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

        let zeros = vec![0u8; POPULATION_SIZE * 4];
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
            (POPULATION_SIZE * 4) as u64,
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

// Fixed I/O positions (must match shader)
const INPUT_A: usize = 0;
const INPUT_B: usize = 1;
const OUTPUT: usize = 12;

fn print_genome(genome: &Genome) {
    println!("\n--- Best Organism (4x4) ---");
    println!("Input A at (0,0), Input B at (1,0), Output at (0,3)\n");
    let op_names = [
        "0", "NOR", "A!B", "!A", "!AB", "!B", "XOR", "NAND",
        "AND", "XNOR", "B", "A|!B", "A", "!A|B", "OR", "1",
        ".", "J"
    ];

    for y in 0..GRID_DIM {
        for x in 0..GRID_DIM {
            let idx = y * GRID_DIM + x;
            let cell = &genome.cells[idx];
            let op_s = if cell.op < 18 {
                op_names[cell.op as usize]
            } else {
                "?"
            };
            let io = if idx == INPUT_A {
                "I"
            } else if idx == INPUT_B {
                "I"
            } else if idx == OUTPUT {
                "O"
            } else {
                " "
            };
            print!("[{:>4}{}] ", op_s, io);
        }
        println!();
    }
}

fn main() {
    println!("========================================");
    println!("   EVOLVE XOR GATE");
    println!("   Input: A, B -> Output: A XOR B");
    println!("   Fitness: correct outputs / 4");
    println!("========================================\n");

    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let gpu = pollster::block_on(GpuContext::new());

    let mut prob_matrix = ProbMatrix::new();
    let mut best_fitness = 0u32;
    let mut best_genome: Option<Genome> = None;
    let mut generations_since_improvement = 0;

    for gen in 0..10000 {
        let population: Vec<Genome> = (0..POPULATION_SIZE)
            .map(|_| prob_matrix.sample(&mut rng))
            .collect();

        let fitness_scores = gpu.evaluate_batch(&population);

        let (best_idx, &gen_best) = fitness_scores
            .iter()
            .enumerate()
            .max_by_key(|(_, &f)| f)
            .unwrap();

        if gen_best > best_fitness {
            best_fitness = gen_best;
            best_genome = Some(population[best_idx].clone());
            generations_since_improvement = 0;
        } else {
            generations_since_improvement += 1;
        }

        let mut indexed: Vec<(usize, u32)> = fitness_scores.iter().cloned().enumerate().collect();
        indexed.sort_by(|a, b| b.1.cmp(&a.1));

        // Update from top performers
        let top_count = (POPULATION_SIZE / 50).max(20);
        for &(idx, _) in indexed.iter().take(top_count) {
            prob_matrix.update(&population[idx]);
        }

        if gen % 10 == 0 || gen_best == MAX_FITNESS as u32 {
            let pct = (best_fitness as f32 / MAX_FITNESS as f32) * 100.0;
            println!(
                "Gen {:>5}: best={}/{} ({:.0}%), stall={}",
                gen, best_fitness, MAX_FITNESS, pct, generations_since_improvement
            );
        }

        if best_fitness == MAX_FITNESS as u32 {
            println!("\n*** PERFECT XOR EVOLVED! ***");
            if let Some(ref g) = best_genome {
                print_genome(g);
            }
            break;
        }

        if generations_since_improvement > 1000 {
            println!("\nStalled for 1000 generations, stopping...");
            break;
        }
    }

    println!("\nFinal best fitness: {}/{} ({:.0}%)",
             best_fitness, MAX_FITNESS,
             (best_fitness as f32 / MAX_FITNESS as f32) * 100.0);

    if let Some(ref g) = best_genome {
        print_genome(g);
    }
}
