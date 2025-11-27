#[path = "../gate_universal.rs"]
mod gate_universal;

use gate_universal::Genome;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use wgpu::{util::DeviceExt, Maintain};
use std::borrow::Cow;
use rayon::prelude::*;

const NUM_NODES: usize = 100;
const INPUT_BITS: usize = 11;
const POPULATION_SIZE: usize = 5000; // Reduced to avoid TDR 
const MAX_GENERATIONS: usize = 5000; // Added max generations

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
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    
    println!("Initializing GPU Quine Search...");
    let gpu = pollster::block_on(GpuContext::new());
    
    let mut population: Vec<Genome> = (0..POPULATION_SIZE)
        .map(|_| Genome::new_random(INPUT_BITS, NUM_NODES, &mut rng))
        .collect();
    
    let mut gen = 0;
    let mut global_best_fitness = u32::MAX;
    let mut global_best_acc = 0.0;
    
    loop {
        if gen >= MAX_GENERATIONS {
            println!("Max generations ({}) reached. Exiting.", MAX_GENERATIONS);
            break;
        }
        let active_genomes: Vec<Genome> = population.par_iter()
            .map(|g| g.get_active_genome())
            .collect();
            
        let errors = gpu.evaluate_batch(&population, &active_genomes);
        
        let mut best_idx = 0;
        let mut min_error = u32::MAX;
        let mut best_len = 1;
        
        for (i, &err) in errors.iter().enumerate() {
            if active_genomes[i].nodes.len() < 20 { continue; } // Minimum complexity
            
            if err < min_error {
                min_error = err;
                best_idx = i;
                best_len = active_genomes[i].to_bits().len();
            }
        }
        
        if min_error != u32::MAX {
            let acc = 1.0 - (min_error as f64 / best_len as f64);
            if acc > global_best_acc { global_best_acc = acc; }
            if min_error < global_best_fitness { global_best_fitness = min_error; }
            
            if gen % 10 == 0 {
                println!("Gen {:04} | Err: {} / {} | Acc: {:.2}% | Best Active: {}", 
                    gen, min_error, best_len, acc * 100.0, active_genomes[best_idx].nodes.len());
            }
            
            if min_error == 0 {
                println!("QUINE SOLVED at Gen {}!", gen);
                println!("GPU reports success. Verifying on CPU...");
                let active_check = population[best_idx].get_active_genome();
                let target_check = active_check.to_bits();
                let mut cpu_error = 0;
                let mut input_bools = vec![false; INPUT_BITS];
                
                println!("Active Nodes: {} | Full Nodes: {} | Target Bits: {}", 
                    active_check.nodes.len(), population[best_idx].nodes.len(), target_check.len());

                for (k, &expected) in target_check.iter().enumerate() {
                    for b in 0..INPUT_BITS { input_bools[b] = (k >> b) & 1 == 1; }
                    if population[best_idx].eval(&input_bools) != expected { cpu_error += 1; }
                }
                
                if cpu_error == 0 {
                    println!("CONFIRMED: QUINE SOLVED at Gen {}!", gen);
                    break;
                } else {
                    println!("FALSE POSITIVE: GPU err 0, CPU err {}", cpu_error);
                    println!("Aborting due to verification failure.");
                    break;
                }
            }
            
            // 4. Reproduction (Elitism + Variable Mutation Spectrum)
            let best_genome = population[best_idx].clone();
            let mut next_gen = Vec::with_capacity(POPULATION_SIZE);
            next_gen.push(best_genome.clone()); // Keep elite
            
            for _ in 1..POPULATION_SIZE {
                let mut child = best_genome.clone();
                let roll = rng.gen::<f64>();
                
                // 5-Tier Mutation Spectrum
                let mut_rate = if roll < 0.10 { 0.001 }      // Low (10%)
                               else if roll < 0.50 { 0.02 }  // Medium (40%)
                               else if roll < 0.90 { 0.10 }  // High (40%)
                               else { 0.50 };                // Extreme (10%)
                
                child.mutate(mut_rate, &mut rng);
                next_gen.push(child);
            }
            population = next_gen;
        } else {
            println!("Extinction! Reseeding...");
            population = (0..POPULATION_SIZE)
                .map(|_| Genome::new_random(INPUT_BITS, NUM_NODES, &mut rng))
                .collect();
        }
        
        gen += 1;
    }
}