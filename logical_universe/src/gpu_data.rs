use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimParams {
    pub width: u32,
    pub height: u32,
    pub depth: u32, // Added depth
    pub offset_x: i32,
    pub offset_y: i32,
    pub offset_z: i32, // Added offset_z
    pub step_type: u32, // 0=Top(AND), 1=Bottom(OR), 2=Top(NOR), 3=Bottom(NAND), 4=Bottom(XOR), 5=Top(XNOR)
    pub frame: u32,
    // Padding to align to 16 bytes? 
    // Current fields: u32, u32, u32, i32, i32, i32, u32, u32 = 8 * 4 bytes = 32 bytes. 
    // 32 is a multiple of 16. No explicit padding array needed if alignment is satisfied naturally.
    // But let's keep explicit padding if needed or just remove it if struct is aligned.
    // Struct size 32. Alignment 4? Storage buffers are loose. Uniforms need 16 byte alignment.
    // 32 is fine.
}
