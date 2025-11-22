use anyhow::{anyhow, Result};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
use wgpu::util::DeviceExt;

use physics_sim::genetic::Genome; 

// Uniforms struct (Rust side) must match WGSL
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    resolution: [f32; 2],
    time: f32,
    _pad1: f32, // Align to 16 bytes
    camera_pos: [f32; 3],
    _pad2: f32, // Align vec3 to 16 bytes
    camera_target: [f32; 3],
    _pad3: f32, // Align vec3 to 16 bytes
    camera_up: [f32; 3],
    _pad4: f32, // Align vec3 to 16 bytes
    genome_params: [f32; 16], 
}

// Main state for the renderer
struct AppState<'a> {
    window: Arc<Window>, // Shared ownership via Arc
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    
    current_genome: Genome,
    start_time: std::time::Instant,
    uniforms_data: Uniforms, 
}

impl<'a> AppState<'a> {
    async fn new(window: Arc<Window>, initial_genome: Genome) -> Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::default();
        
        // Create surface using Arc<Window>
        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter"); 

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                    ..Default::default()
                },
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb()) 
            .next()
            .unwrap_or(surface_caps.formats[0]); 

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // --- Shader and Pipeline ---
        let shader = device.create_shader_module(wgpu::include_wgsl!("render_shader.wgsl"));

        let initial_uniforms = Uniforms {
            resolution: [size.width as f32, size.height as f32],
            time: 0.0,
            _pad1: 0.0,
            camera_pos: [0.0, 0.0, -10.0],
            _pad2: 0.0,
            camera_target: [0.0, 0.0, 0.0],
            _pad3: 0.0,
            camera_up: [0.0, 1.0, 0.0],
            _pad4: 0.0,
            genome_params: initial_genome.genes,
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[initial_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT, 
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
        });


        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"), 
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"), 
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip, 
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false, 
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            uniform_buffer,
            bind_group,
            current_genome: initial_genome,
            start_time: std::time::Instant::now(),
            uniforms_data: initial_uniforms,
        })
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.uniforms_data.resolution = [new_size.width as f32, new_size.height as f32];
            self.queue.write_buffer(
                &self.uniform_buffer,
                0, 
                bytemuck::cast_slice(&[self.uniforms_data]),
            );
        }
    }

    fn update(&mut self) {
        let current_time = self.start_time.elapsed().as_secs_f32();
        self.uniforms_data.time = current_time;
        self.uniforms_data.camera_pos = [current_time.sin() * 5.0, current_time.cos() * 2.0, -10.0 + current_time.sin() * 3.0];
        
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms_data]),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None, 
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..4, 0..1); 
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

// Wrapper application struct to handle the optional state
struct App<'a> {
    state: Option<AppState<'a>>,
}

impl<'a> ApplicationHandler for App<'a> { 
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop.create_window(Window::default_attributes().with_title("3D Genome Visualizer")).unwrap()
        );
        
        let initial_genome = Genome::random();
        // Need to block on async init
        let state = pollster::block_on(AppState::new(window, initial_genome)).unwrap();
        self.state = Some(state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if let Some(state) = self.state.as_mut() {
            if state.window.id() == window_id {
                match event {
                    WindowEvent::CloseRequested => event_loop.exit(),
                    WindowEvent::Resized(size) => state.resize(size),
                    WindowEvent::RedrawRequested => {
                        state.update();
                        match state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => state.resize(state.size),
                            Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                            Err(e) => eprintln!("Render error: {:?}", e),
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    
    let mut app = App { state: None };
    event_loop.run_app(&mut app)?;
    Ok(())
}