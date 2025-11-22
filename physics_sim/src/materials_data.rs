use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuAtom {
    pub position: [f32; 4], // x, y, z, mass
    pub velocity: [f32; 4], // vx, vy, vz, element_id
    pub forces: [f32; 4],   // fx, fy, fz, padding
}

impl GpuAtom {
    pub fn new(x: f32, y: f32, z: f32, element_id: f32, mass: f32) -> Self {
        Self {
            position: [x, y, z, mass],
            velocity: [0.0, 0.0, 0.0, element_id],
            forces: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MaterialSimParams {
    pub dt: f32,
    pub box_size: [f32; 3],
    pub gravity: f32,
    pub pull_force: f32, // Stress test force
    pub padding: [f32; 2],
}
