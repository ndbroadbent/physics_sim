use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimParams {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub offset_z: i32,
    pub step_type: u32,
    pub frame: u32,
    pub wrap: u32,
    pub _pad: [u32; 3],
}
