use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimParams {
    pub width: u32,
    pub height: u32,
    pub step_type: u32, // 0 = Update Top (AND/NOR), 1 = Update Bottom (OR/NAND)
    pub frame: u32,
    pub _padding: [u32; 4], 
}
