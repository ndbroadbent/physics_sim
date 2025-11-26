pub const IMAGE_WIDTH: u32 = 256;
pub const IMAGE_HEIGHT: u32 = 256;

// Total pixels
pub const TOTAL_PIXELS: usize = (IMAGE_WIDTH * IMAGE_HEIGHT) as usize;
// We process 64 pixels per 'chunk' (u64)
pub const CHUNK_SIZE: usize = 64;
pub const NUM_CHUNKS: usize = TOTAL_PIXELS / CHUNK_SIZE;

// How many bits do we use for coordinates? 8 bits for 256 size.
pub const COORD_BITS: usize = 8;

pub struct PrecomputedInputs {
    // For every chunk, we have 'COORD_BITS' u64s for X, and 'COORD_BITS' u64s for Y.
    // Structure: [chunk_index][bit_index]
    pub x_bits: Vec<Vec<u64>>, 
    pub y_bits: Vec<Vec<u64>>,
}

impl PrecomputedInputs {
    pub fn new() -> Self {
        let mut x_bits = vec![vec![0u64; COORD_BITS]; NUM_CHUNKS];
        let mut y_bits = vec![vec![0u64; COORD_BITS]; NUM_CHUNKS];

        for chunk_idx in 0..NUM_CHUNKS {
            for i in 0..CHUNK_SIZE {
                let global_pixel_idx = chunk_idx * CHUNK_SIZE + i;
                let x = (global_pixel_idx as u32 % IMAGE_WIDTH) as u8;
                let y = (global_pixel_idx as u32 / IMAGE_WIDTH) as u8;

                // Distribute the bits of x and y into the SoA structure
                for b in 0..COORD_BITS {
                    let bit_val_x = ((x >> b) & 1) as u64;
                    let bit_val_y = ((y >> b) & 1) as u64;

                    x_bits[chunk_idx][b] |= bit_val_x << i;
                    y_bits[chunk_idx][b] |= bit_val_y << i;
                }
            }
        }

        PrecomputedInputs { x_bits, y_bits }
    }
}
