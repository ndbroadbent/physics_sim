use crate::inputs::{IMAGE_WIDTH, IMAGE_HEIGHT, NUM_CHUNKS, CHUNK_SIZE};

pub struct Target {
    // Packed bits: 1 = black/on, 0 = white/off
    pub expected_output: Vec<u64>, 
}

impl Target {
    pub fn circle(radius: f32) -> Self {
        let mut data = vec![0u64; NUM_CHUNKS];
        let cx = IMAGE_WIDTH as f32 / 2.0;
        let cy = IMAGE_HEIGHT as f32 / 2.0;
        let r_sq = radius * radius;

        for chunk_idx in 0..NUM_CHUNKS {
            for i in 0..CHUNK_SIZE {
                let global_pixel_idx = chunk_idx * CHUNK_SIZE + i;
                let x = (global_pixel_idx as u32 % IMAGE_WIDTH) as f32;
                let y = (global_pixel_idx as u32 / IMAGE_WIDTH) as f32;

                let dx = x - cx;
                let dy = y - cy;
                if dx*dx + dy*dy < r_sq {
                    data[chunk_idx] |= 1 << i;
                }
            }
        }
        Target { expected_output: data }
    }

    pub fn square(half_width: f32) -> Self {
        let mut data = vec![0u64; NUM_CHUNKS];
        let cx = IMAGE_WIDTH as f32 / 2.0;
        let cy = IMAGE_HEIGHT as f32 / 2.0;

        for chunk_idx in 0..NUM_CHUNKS {
            for i in 0..CHUNK_SIZE {
                let global_pixel_idx = chunk_idx * CHUNK_SIZE + i;
                let x = (global_pixel_idx as u32 % IMAGE_WIDTH) as f32;
                let y = (global_pixel_idx as u32 / IMAGE_WIDTH) as f32;

                if (x - cx).abs() < half_width && (y - cy).abs() < half_width {
                    data[chunk_idx] |= 1 << i;
                }
            }
        }
        Target { expected_output: data }
    }
}
