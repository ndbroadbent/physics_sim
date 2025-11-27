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
const POPULATION_SIZE: usize = 100; // Matched loop

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
        
        for (g, active) in genomes.iter().zip(active_genomes.iter()) {
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
            raw_nodes.extend(g_nodes);

            let bits = active.to_bits();
            let len_bits = bits.len() as u32;
            let mut current_word = 0u32;
            let mut bit_count = 0;
            let mut packed = Vec::new();
            
            for &b in &bits {
                if b { current_word |= 1 << bit_count; }
                bit_count += 1;
                if bit_count == 32 {
                    packed.push(current_word);
                    current_word = 0;
                    bit_count = 0;
                }
            }
            if bit_count > 0 { packed.push(current_word); }

            let start_bit = (raw_targets.len() * 32) as u32;
            raw_targets.extend(packed);
            
            raw_infos.push(GenomeInfo {
                output_node_idx: g.output_node as u32,
                target_start_bit: start_bit,
                target_len_bits: len_bits,
                _pad: 0,
            });
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
    let mut rng = ChaCha8Rng::seed_from_u64(12345);
    
    println!("Initializing GPU Debug (Stress Test)...");
    let gpu = pollster::block_on(GpuContext::new());
    
    // Create Population
    let mut population = Vec::new();
    for _ in 0..POPULATION_SIZE {
        let mut g = Genome::new_random(INPUT_BITS, NUM_NODES, &mut rng);
        g.mutate(0.10, &mut rng);
        population.push(g);
    }
    
    let active_genomes: Vec<Genome> = population.iter().map(|g| g.get_active_genome()).collect();
    
    // GPU Eval
    let gpu_errors = gpu.evaluate_batch(&population, &active_genomes);
    
    let mut mismatches = 0;
    
    for (i, g) in population.iter().enumerate() {
        let active = &active_genomes[i];
        
        if active.nodes.len() < 20 { continue; } 
        
        let target = active.to_bits();
        let mut cpu_error = 0;
        let mut input_bools = vec![false; INPUT_BITS];
        for (k, &expected) in target.iter().enumerate() {
            for b in 0..INPUT_BITS { input_bools[b] = (k >> b) & 1 == 1; }
            if g.eval(&input_bools) != expected { cpu_error += 1; }
        }
        
        let gpu_error = gpu_errors[i];
        
        if cpu_error as u32 != gpu_error {
            println!("MISMATCH at idx {}: CPU {} != GPU {}", i, cpu_error, gpu_error);
            mismatches += 1;
        }
    }
    
    if mismatches == 0 {
        println!("SUCCESS: All tested genomes matched CPU/GPU evaluation.");
    } else {
        println!("FAILURE: {} mismatches detected.", mismatches);
    }
}
