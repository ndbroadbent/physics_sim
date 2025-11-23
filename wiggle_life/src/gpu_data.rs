use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimParams {
    pub width: u32,
    pub height: u32,
    pub step_type: u32, // 0 = Update Top (AND/NOR), 1 = Update Bottom (OR/NAND)
    pub _padding: [u32; 5], // Aligned to 16 bytes (8 floats/u32s total size usually required for uniform buffers in some alignments, but here 3+5=8 u32s = 32 bytes is safe)
}
