pub struct GpuParticle {
    pub position: [f32; 4], 
    pub velocity: [f32; 4], 
    pub properties: [f32; 4], 
}

impl GpuParticle {
    pub const TYPE_PHOTON: f32 = 0.0;
    pub const TYPE_ELECTRON: f32 = 1.0;
    pub const TYPE_HOLE: f32 = 2.0;

    pub fn new_photon(x: f32, y: f32, energy: f32) -> Self {
        Self {
            position: [x, y, 0.0, 0.0],
            velocity: [1.0, 0.0, 0.0, 0.0], 
            properties: [energy, Self::TYPE_PHOTON, 1.0, 0.0],
        }
    }
}

// Expanded SimParams to support up to 4 layers
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimParams {
    pub width: f32,
    pub height: f32,
    pub atom_density: f32,
    pub dt: f32,
    
    // Layer 1
    pub l1_start: f32,
    pub l1_end: f32,
    pub l1_band_gap: f32,
    pub l1_active: f32, // 1.0 if active

    // Layer 2
    pub l2_start: f32,
    pub l2_end: f32,
    pub l2_band_gap: f32,
    pub l2_active: f32,

    // Layer 3
    pub l3_start: f32,
    pub l3_end: f32,
    pub l3_band_gap: f32,
    pub l3_active: f32,

    pub padding: [f32; 4], // Pad to 16-byte alignment
}


// New type for genetic simulation particles
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuMagneticParticle {
    pub position: [f32; 4], // x, y, z, padding/chamber_id
    pub velocity: [f32; 4], // vx, vy, vz, padding
    pub magnetic_moment: [f32; 4], // mx, my, mz, padding (or just magnitude)
    pub properties: [f32; 4], // current_flow_accum, active_flag, padding, padding
}

impl GpuMagneticParticle {
    pub fn new(x: f32, y: f32, z: f32, chamber_id: f32) -> Self {
        Self {
            position: [x, y, z, chamber_id],
            velocity: [0.0, 0.0, 0.0, 0.0],
            magnetic_moment: [1.0, 0.0, 0.0, 0.0], // Default magnetic moment
            properties: [0.0, 1.0, 0.0, 0.0], // flow_accum, active=true
        }
    }
}

// GeneticSimParams struct (Rust side) must match WGSL
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeneticSimParams {
    pub dt: f32,
    pub temp_gradient_start_x: f32,
    pub temp_gradient_end_x: f32,
    pub max_temp_noise_mag: f32, 
    
    pub chamber_dims: [f32; 3],
    pub _pad_chamber_dims: f32, // Explicit padding to make chamber_dims a vec4 equivalent (16 bytes)

    pub genome_genes: [f32; 16], // 16 genes, treated as 4 vec4s in WGSL
}