use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
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
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
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

    

        pub padding: [f32; 4], // Pad to 80 bytes (16-byte alignment)

    }

    