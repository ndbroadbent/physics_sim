#[path = "../gate_universal.rs"]
mod gate_universal;

use gate_universal::{Genome, UniversalNode};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use wgpu::{util::DeviceExt, Maintain};
use std::borrow::Cow;
use rayon::prelude::*;

const NUM_NODES: usize = 50; // Target small quine
const INPUT_BITS: usize = 11;
const TOTAL_INPUTS: usize = INPUT_BITS + NUM_NODES;
const POPULATION_SIZE: usize = 10000; 
const LEARNING_RATE: f32 = 0.05;
const NEGATIVE_RATE: f32 = 0.01; // Penalty for bad solutions?

// GPU Structures (Same as before)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuNode {
    table: u32,
    in_a: u32,
    in_b: u32,
    _pad: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GenomeInfo {
    output_node_idx: u32,
    target_start_bit: u32,
    target_len_bits: u32,
    _pad: u32,
}

// Probability Distribution
struct ProbMatrix {
    // [Node][Op (0..16)]
    op_probs: Vec<Vec<f32>>,
    // [Node][Input (0..TOTAL_INPUTS)]
    in_a_probs: Vec<Vec<f32>>,
    in_b_probs: Vec<Vec<f32>>,
    // Output node probability
    out_probs: Vec<f32>,
}

impl ProbMatrix {
    fn new() -> Self {
        // Initialize with uniform probabilities
        let op_uniform = 1.0 / 16.0;
        let in_uniform = 1.0 / TOTAL_INPUTS as f32;
        let out_uniform = 1.0 / TOTAL_INPUTS as f32;

        ProbMatrix {
            op_probs: vec![vec![op_uniform; 16]; NUM_NODES],
            in_a_probs: vec![vec![in_uniform; TOTAL_INPUTS]; NUM_NODES],
            in_b_probs: vec![vec![in_uniform; TOTAL_INPUTS]; NUM_NODES],
            out_probs: vec![out_uniform; TOTAL_INPUTS],
        }
    }

    fn sample(&self, rng: &mut impl Rng) -> Genome {
        let mut nodes = Vec::with_capacity(NUM_NODES);
        
        for i in 0..NUM_NODES {
            // Sample Op
            let op_idx = sample_discrete(&self.op_probs[i], rng);
            
            // Sample Inputs (Allowed to look at any previous node or input)
            // We enforce DAG by masking probabilities? 
            // Or just let it sample and if it picks invalid, we modulo?
            // Better: Only sample from allowed range 0..(INPUT_BITS + i)
            // But to keep matrix simple, we sample from 0..TOTAL_INPUTS and just reject/retry?
            // Or simpler: Mask the probabilities during sampling.
            
            let limit = INPUT_BITS + i;
            
            let in_a = sample_discrete_masked(&self.in_a_probs[i], limit, rng);
            let in_b = sample_discrete_masked(&self.in_b_probs[i], limit, rng);
            
            nodes.push(UniversalNode {
                table: op_idx as u8,
                in_a: in_a as u16,
                in_b: in_b as u16,
            });
        }
        
        let output_node = sample_discrete(&self.out_probs, rng);
        
        Genome {
            num_inputs: INPUT_BITS,
            nodes,
            output_node,
        }
    }

    fn update(&mut self, best_genome: &Genome) {
        let one_minus_lr = 1.0 - LEARNING_RATE;

        for (i, node) in best_genome.nodes.iter().enumerate() {
            // Op
            let op = node.table as usize;
            for j in 0..16 {
                self.op_probs[i][j] *= one_minus_lr;
            }
            self.op_probs[i][op] += LEARNING_RATE;
            normalize(&mut self.op_probs[i]);
            
            // In A
            let a = node.in_a as usize;
            for j in 0..TOTAL_INPUTS {
                self.in_a_probs[i][j] *= one_minus_lr;
            }
            self.in_a_probs[i][a] += LEARNING_RATE;
            normalize(&mut self.in_a_probs[i]);
            
            // In B
            let b = node.in_b as usize;
            for j in 0..TOTAL_INPUTS {
                self.in_b_probs[i][j] *= one_minus_lr;
            }
            self.in_b_probs[i][b] += LEARNING_RATE;
            normalize(&mut self.in_b_probs[i]);
        }
        
        // Output node
        let out = best_genome.output_node;
        for j in 0..TOTAL_INPUTS {
            self.out_probs[j] *= one_minus_lr;
        }
        self.out_probs[out] += LEARNING_RATE;
        normalize(&mut self.out_probs);
    }
    
    fn inject_noise(&mut self, amount: f32) {
        let uniform_op = 1.0 / 16.0;
        let uniform_in = 1.0 / TOTAL_INPUTS as f32;
        
        for row in &mut self.op_probs {
            for p in row.iter_mut() {
                *p = *p * (1.0 - amount) + uniform_op * amount;
            }
            normalize(row); // Re-normalize after noise
        }
        for row in &mut self.in_a_probs {
            for p in row.iter_mut() {
                *p = *p * (1.0 - amount) + uniform_in * amount;
            }
            normalize(row);
        }
        for row in &mut self.in_b_probs {
            for p in row.iter_mut() {
                *p = *p * (1.0 - amount) + uniform_in * amount;
            }
            normalize(row);
        }
        for p in self.out_probs.iter_mut() {
            *p = *p * (1.0 - amount) + uniform_in * amount;
        }
        normalize(&mut self.out_probs);
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

fn sample_discrete_masked(probs: &[f32], limit: usize, rng: &mut impl Rng) -> usize {
    // Sample only from indices < limit
    // Re-normalize on the fly
    let mut sum_valid = 0.0;
    for i in 0..limit { sum_valid += probs[i]; }
    
    let r = rng.gen::<f32>() * sum_valid;
    let mut sum = 0.0;
    for i in 0..limit {
        sum += probs[i];
        if r < sum { return i; }
    }
    limit - 1
}

fn normalize(probs: &mut Vec<f32>) {
    let sum: f32 = probs.iter().sum();
    for p in probs.iter_mut() { *p /= sum; }
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
    genome_buffer: wgpu::Buffer,
    info_buffer: wgpu::Buffer,
    targets_buffer: wgpu::Buffer,
    error_buffer: wgpu::Buffer,
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
            label: Some("Quine Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../quine_evaluator.wgsl"))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
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

        let genome_size = (POPULATION_SIZE * NUM_NODES * std::mem::size_of::<GpuNode>()) as u64;
        let genome_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Genome"), size: genome_size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

        let info_size = (POPULATION_SIZE * std::mem::size_of::<GenomeInfo>()) as u64;
        let info_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Info"), size: info_size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

        let max_target_bits = POPULATION_SIZE * 2048; 
        let targets_size = ((max_target_bits / 32) * 4) as u64;
        let targets_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Targets"), size: targets_size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

        let error_size = (POPULATION_SIZE * 4) as u64;
        let error_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Errors"), size: error_size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("Staging"), size: error_size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

        GpuContext { device, queue, pipeline, bind_group_layout, genome_buffer, info_buffer, targets_buffer, error_buffer, staging_buffer }
    }

    fn evaluate_batch(&self, genomes: &[Genome], active_genomes: &[Genome]) -> Vec<u32> {
        let batch_size = genomes.len();
        let mut raw_nodes = Vec::with_capacity(batch_size * NUM_NODES);
        let mut raw_targets = Vec::new();
        let mut raw_infos = Vec::with_capacity(batch_size);
        
        let prepared_data: Vec<(Vec<GpuNode>, Vec<u32>, u32)> = genomes.par_iter().zip(active_genomes.par_iter()).map(|(g, active)| {
            let mut g_nodes = Vec::with_capacity(NUM_NODES);
            for n in &g.nodes {
                g_nodes.push(GpuNode {
                    table: n.table as u32,
                    in_a: n.in_a as u32,
                    in_b: n.in_b as u32,
                    _pad: 0,
                });
            }
            while g_nodes.len() < NUM_NODES {
                g_nodes.push(GpuNode { table: 0, in_a: 0, in_b: 0, _pad: 0 });
            }

            let bits = active.to_bits();
            let len_bits = bits.len() as u32;
            let mut packed_targets = Vec::new();
            let mut current_word = 0u32;
            let mut bit_count = 0;
            
            for &b in &bits {
                if b { current_word |= 1 << bit_count; }
                bit_count += 1;
                if bit_count == 32 {
                    packed_targets.push(current_word);
                    current_word = 0;
                    bit_count = 0;
                }
            }
            if bit_count > 0 { packed_targets.push(current_word); }

            (g_nodes, packed_targets, len_bits)
        }).collect();

        for (nodes, targets, len) in prepared_data {
            raw_nodes.extend(nodes);
            let start_bit = (raw_targets.len() * 32) as u32;
            raw_targets.extend(targets);
            raw_infos.push(GenomeInfo {
                output_node_idx: 0, 
                target_start_bit: start_bit,
                target_len_bits: len,
                _pad: 0,
            });
        }
        
        for (i, g) in genomes.iter().enumerate() {
            raw_infos[i].output_node_idx = g.output_node as u32;
        }

        self.queue.write_buffer(&self.genome_buffer, 0, bytemuck::cast_slice(&raw_nodes));
        self.queue.write_buffer(&self.info_buffer, 0, bytemuck::cast_slice(&raw_infos));
        self.queue.write_buffer(&self.targets_buffer, 0, bytemuck::cast_slice(&raw_targets));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.genome_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: self.info_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: self.targets_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: self.error_buffer.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (batch_size + 63) / 64;
            cpass.dispatch_workgroups(workgroups as u32, 1, 1);
        }

        encoder.copy_buffer_to_buffer(&self.error_buffer, 0, &self.staging_buffer, 0, (batch_size * 4) as u64);
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = self.staging_buffer.slice(0..(batch_size * 4) as u64);
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
    let mut rng = ChaCha8Rng::seed_from_u64(999);
    println!("Initializing Diffusion Quine Search...");
    let gpu = pollster::block_on(GpuContext::new());
    
    // Initialize Probability Matrix (The Cloud)
    let mut prob_matrix = ProbMatrix::new();
    
    let mut gen = 0;
    let mut global_best_error = u32::MAX;
    
    loop {
        if gen >= 200 { // Max 200 generations
            println!("Max generations (200) reached. Exiting.");
            break;
        }
        // 1. Sample Population from Cloud
        let population: Vec<Genome> = (0..POPULATION_SIZE)
            .map(|_| prob_matrix.sample(&mut rng))
            .collect();
            
        // 2. Evaluate
        let active_genomes: Vec<Genome> = population.par_iter()
            .map(|g| g.get_active_genome())
            .collect();
            
        let errors = gpu.evaluate_batch(&population, &active_genomes);
        
        // 3. Update (Reinforce best)
        // Sort to find best
        let mut results: Vec<(usize, u32)> = errors.into_iter().enumerate().collect();
        results.sort_by_key(|&(_, err)| err);
        
        // Pick top 1% to reinforce
        let elite_count = POPULATION_SIZE / 100;
        
        // Also filter for reasonable size?
        let mut valid_elites = 0;
        
        // Stats tracking
        let mut batch_best_err = u32::MAX;
        let mut batch_best_active = 0;
        let mut active_sum = 0;
        let mut valid_samples = 0;

        for i in 0..elite_count {
            let (idx, raw_err) = results[i];
            let active_size = active_genomes[idx].nodes.len();
            
            // STRICTLY SKIP active size 0
            if active_size == 0 { continue; }
            
            active_sum += active_size;
            valid_samples += 1;
            
            // Soft Penalty for small size (still useful for ranking valid ones)
            let size_penalty = if active_size < 20 { (20 - active_size) as u32 * 100 } else { 0 };
            let adjusted_err = raw_err + size_penalty;
            
            prob_matrix.update(&population[idx]);
            valid_elites += 1;
            
            if adjusted_err < batch_best_err {
                batch_best_err = adjusted_err;
                batch_best_active = active_size;
            }
            
            if adjusted_err < global_best_error {
                global_best_error = adjusted_err;
                let len = active_genomes[idx].to_bits().len();
                let safe_len = if len == 0 { 1 } else { len };
                let acc = 1.0 - (raw_err as f64 / safe_len as f64);
                println!("NEW BEST: Gen {} | Err {} (Adj {}) / {} | Acc {:.2}% | Active {}", 
                    gen, raw_err, adjusted_err, len, acc * 100.0, active_size);
                
                if raw_err == 0 {
                    println!("QUINE SOLVED!");
                    return;
                }
            }
        }
        
        // Entropy Maintenance
        let entropy = prob_matrix.entropy();
        if entropy < 10.0 {
            // Reheat!
            // println!("Entropy low ({:.2}), reheating...");
            prob_matrix.inject_noise(0.05);
        }
        
        if gen % 10 == 0 {
            let avg_active = if valid_samples > 0 { active_sum as f64 / valid_samples as f64 } else { 0.0 };
            println!("Gen {:04} | Ent: {:.2} | Elites: {} | Batch Best: Err {} (Act {}) | Avg Act: {:.1}", 
                gen, entropy, valid_elites, batch_best_err, batch_best_active, avg_active);
        }
        
        gen += 1;
        if gen > 5000 { break; }
    }
}
