use bytemuck::{Pod, Zeroable};

/// GPU-Ready Particle Structure
/// Aligned to 16-byte boundaries for WGSL compatibility.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuParticle {
    pub position: [f32; 4], // x, y, z, padding/mass
    pub velocity: [f32; 4], // vx, vy, vz, padding/charge
    pub properties: [f32; 4], // energy, type_id, active_flag, padding
}

impl GpuParticle {
    // Particle Types (match WGSL)
    pub const TYPE_PHOTON: f32 = 0.0;
    pub const TYPE_ELECTRON: f32 = 1.0;
    pub const TYPE_HOLE: f32 = 2.0;

    pub fn new_photon(x: f32, y: f32, energy: f32) -> Self {
        Self {
            position: [x, y, 0.0, 0.0],
            velocity: [1.0, 0.0, 0.0, 0.0], // Moving right at c=1
            properties: [energy, Self::TYPE_PHOTON, 1.0, 0.0],
        }
    }
}

// Ensure this matches the WGSL SimParams struct and is 16-byte aligned
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SimParams {
    pub width: f32,
    pub height: f32,
    pub atom_density: f32,
    pub band_gap: f32,
    pub dt: f32,
    pub padding: [f32; 3], // Ensures 16-byte alignment (5*4 + 3*4 = 32 bytes)
}
