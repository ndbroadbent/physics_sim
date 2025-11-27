use crate::inputs::{IMAGE_WIDTH, IMAGE_HEIGHT, NUM_CHUNKS, CHUNK_SIZE};

pub struct Target {
    // Packed bits: 1 = black/on, 0 = white/off
    pub expected_output: Vec<u32>, // Changed to u32 to match GPU
}

impl Target {
    pub fn circle(radius: f32) -> Self {
        let mut data = vec![0u32; NUM_CHUNKS * 2]; // Double chunks for GPU (32-bit vs 64-bit split)
        let cx = IMAGE_WIDTH as f32 / 2.0;
        let cy = IMAGE_HEIGHT as f32 / 2.0;
        let r_sq = radius * radius;

        // CPU uses 64-bit chunks (NUM_CHUNKS). GPU uses 32-bit chunks (NUM_CHUNKS * 2).
        // Let's fill the GPU format directly.
        // 256x256 = 65536 pixels.
        // 32 bits per u32. Total u32s = 2048.
        
        for idx in 0..(NUM_CHUNKS * 2) {
            for bit in 0..32 {
                let global_pixel_idx = idx * 32 + bit;
                if global_pixel_idx >= (IMAGE_WIDTH * IMAGE_HEIGHT) as usize { break; }
                
                let x = (global_pixel_idx as u32 % IMAGE_WIDTH) as f32;
                let y = (global_pixel_idx as u32 / IMAGE_WIDTH) as f32;

                let dx = x - cx;
                let dy = y - cy;
                if dx*dx + dy*dy < r_sq {
                    data[idx] |= 1 << bit;
                }
            }
        }
        Target { expected_output: data }
    }

    pub fn square(half_width: f32) -> Self {
        let mut data = vec![0u32; NUM_CHUNKS * 2];
        let cx = IMAGE_WIDTH as f32 / 2.0;
        let cy = IMAGE_HEIGHT as f32 / 2.0;

        for idx in 0..(NUM_CHUNKS * 2) {
            for bit in 0..32 {
                let global_pixel_idx = idx * 32 + bit;
                if global_pixel_idx >= (IMAGE_WIDTH * IMAGE_HEIGHT) as usize { break; }
                
                let x = (global_pixel_idx as u32 % IMAGE_WIDTH) as f32;
                let y = (global_pixel_idx as u32 / IMAGE_WIDTH) as f32;

                if (x - cx).abs() < half_width && (y - cy).abs() < half_width {
                    data[idx] |= 1 << bit;
                }
            }
        }
        Target { expected_output: data }
    }

    pub fn checkerboard(square_size: u32) -> Self {
        let mut data = vec![0u32; NUM_CHUNKS * 2];
        
        for idx in 0..(NUM_CHUNKS * 2) {
            for bit in 0..32 {
                let global_pixel_idx = idx * 32 + bit;
                if global_pixel_idx >= (IMAGE_WIDTH * IMAGE_HEIGHT) as usize { break; }
                
                let x = (global_pixel_idx as u32 % IMAGE_WIDTH);
                let y = (global_pixel_idx as u32 / IMAGE_WIDTH);

                let x_cell = x / square_size;
                let y_cell = y / square_size;
                
                if (x_cell % 2) == (y_cell % 2) {
                    data[idx] |= 1 << bit;
                }
            }
        }
        Target { expected_output: data }
    }
}