use wgpu::util::DeviceExt;
use crate::gate::{Genome, Operation};
use crate::inputs::{PrecomputedInputs, NUM_CHUNKS};
use crate::targets::Target;
use std::borrow::Cow;

// Must match WGSL
const NUM_NODES: usize = 600;
const NUM_INPUTS: usize = 17; 
const CHUNKS_PER_IMAGE: usize = NUM_CHUNKS;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuNode {
    op: u32,
    in_a: u32,
    in_b: u32,
    _pad: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GenomeInfo {
    output_node_idx: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

pub struct GpuEvaluator {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    
    // Persistent Buffers
    inputs_buffer: wgpu::Buffer,
    target_buffer: wgpu::Buffer,
    
    // Reusable Buffers (Resized as needed)
    genome_buffer: Option<wgpu::Buffer>,
    info_buffer: Option<wgpu::Buffer>,
    error_buffer: Option<wgpu::Buffer>,
    staging_buffer: Option<wgpu::Buffer>,
    
    current_batch_size: usize,
}

impl GpuEvaluator {
    pub async fn new(inputs: &PrecomputedInputs, target: &Target) -> Self {
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

        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Evaluator Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("evaluator.wgsl"))),
        });

        // Upload Inputs (Flattened)
        // Inputs: [Chunk0_In0..16, Chunk1_In0..16]
        // We emulate 64-bit inputs by splitting into two 32-bit chunks
        
        let mut gpu_inputs = Vec::with_capacity(NUM_CHUNKS * 2 * NUM_INPUTS);
        for i in 0..NUM_CHUNKS {
            // Low 32 bits
            for b in 0..8 { gpu_inputs.push(inputs.x_bits[i][b] as u32); }
            for b in 0..8 { gpu_inputs.push(inputs.y_bits[i][b] as u32); }
            gpu_inputs.push(u32::MAX); // ShapeID=1 Low
            
            // High 32 bits
            for b in 0..8 { gpu_inputs.push((inputs.x_bits[i][b] >> 32) as u32); }
            for b in 0..8 { gpu_inputs.push((inputs.y_bits[i][b] >> 32) as u32); }
            gpu_inputs.push(u32::MAX); // ShapeID=1 High
        }
        
        // Upload Target (Same Split)
        let mut gpu_target = Vec::with_capacity(NUM_CHUNKS * 2);
        for i in 0..NUM_CHUNKS {
            gpu_target.push(target.expected_output[i] as u32);
            gpu_target.push((target.expected_output[i] >> 32) as u32);
        }

        let inputs_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Inputs Buffer"),
            contents: bytemuck::cast_slice(&gpu_inputs),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let target_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Target Buffer"),
            contents: bytemuck::cast_slice(&gpu_target),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Bind Group Layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Layout"),
            entries: &[
                // 0: Genomes
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                // 1: Info
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                // 2: Target
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                // 3: Inputs
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                // 4: Errors
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
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

        GpuEvaluator {
            device, queue, pipeline, bind_group_layout,
            inputs_buffer, target_buffer,
            genome_buffer: None, info_buffer: None, error_buffer: None, staging_buffer: None,
            current_batch_size: 0,
        }
    }

    pub fn evaluate_batch(&mut self, genomes: &[Genome]) -> Vec<u64> {
        let batch_size = genomes.len();
        
        // Resize buffers if needed
        if batch_size > self.current_batch_size {
            self.current_batch_size = batch_size;
            
            let genome_size = (batch_size * NUM_NODES * std::mem::size_of::<GpuNode>()) as u64;
            self.genome_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Genome Buffer"),
                size: genome_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));

            let info_size = (batch_size * std::mem::size_of::<GenomeInfo>()) as u64;
            self.info_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Info Buffer"),
                size: info_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));

            let error_size = (batch_size * std::mem::size_of::<u32>()) as u64;
            self.error_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Error Buffer"),
                size: error_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST, // Need COPY_DST to clear it
                mapped_at_creation: false,
            }));
            
            self.staging_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Staging Buffer"),
                size: error_size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }

        // Prepare Data
        let mut raw_nodes = Vec::with_capacity(batch_size * NUM_NODES);
        let mut raw_infos = Vec::with_capacity(batch_size);

        for g in genomes {
            // Convert Genome to GpuNodes
            for n in &g.nodes {
                raw_nodes.push(GpuNode {
                    op: match n.op {
                        Operation::AND => 0, Operation::OR => 1, Operation::XOR => 2,
                        Operation::NAND => 3, Operation::NOR => 4, Operation::NOT => 5,
                        Operation::WIRE => 6,
                    },
                    in_a: n.in_a as u32,
                    in_b: n.in_b as u32,
                    _pad: 0,
                });
            }
            // Fill remaining if genome is smaller than MAX? (Assuming fixed 600)
            
            raw_infos.push(GenomeInfo {
                output_node_idx: g.output_node_idx as u32,
                _pad1: 0, _pad2: 0, _pad3: 0,
            });
        }

        // Upload
        self.queue.write_buffer(self.genome_buffer.as_ref().unwrap(), 0, bytemuck::cast_slice(&raw_nodes));
        self.queue.write_buffer(self.info_buffer.as_ref().unwrap(), 0, bytemuck::cast_slice(&raw_infos));
        
        // Clear Error Buffer
        let zeros = vec![0u32; batch_size];
        self.queue.write_buffer(self.error_buffer.as_ref().unwrap(), 0, bytemuck::cast_slice(&zeros));

        // Bind Group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.genome_buffer.as_ref().unwrap().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: self.info_buffer.as_ref().unwrap().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: self.target_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: self.inputs_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: self.error_buffer.as_ref().unwrap().as_entire_binding() },
            ],
        });

        // Dispatch
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            
            // Dispatch (2D)
            // X: Chunks (2048 / 64 = 32 workgroups)
            // Y: Genomes (batch_size workgroups)
            let chunks_per_image = 2048;
            let workgroups_x = (chunks_per_image + 63) / 64;
            cpass.dispatch_workgroups(workgroups_x as u32, batch_size as u32, 1);
        }

        // Readback
        encoder.copy_buffer_to_buffer(self.error_buffer.as_ref().unwrap(), 0, self.staging_buffer.as_ref().unwrap(), 0, (batch_size * 4) as u64);
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = self.staging_buffer.as_ref().unwrap().slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        self.staging_buffer.as_ref().unwrap().unmap();

        result.into_iter().map(|x| x as u64).collect()
    }
}