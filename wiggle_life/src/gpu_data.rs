use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimParams {
    pub width: u32,
    pub height: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub step_type: u32, // 0=Top(AND), 1=Bottom(OR), 2=Top(NOR), 3=Bottom(NAND), 4=Bottom(XOR), 5=Top(XNOR)
    pub frame: u32,
    pub _padding: [u32; 2],
}
