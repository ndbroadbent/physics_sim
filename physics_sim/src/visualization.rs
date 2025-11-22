use image::{RgbImage, Rgb};

pub struct SimulationVisualizer {
    width: u32,
    height: u32,
    buffer: RgbImage,
}

impl SimulationVisualizer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            buffer: RgbImage::new(width, height),
        }
    }

    pub fn draw_atom(&mut self, x: f64, y: f64) {
        let px = x as u32;
        let py = y as u32;
        
        // Draw a small white circle for the atom
        for i in 0..3 {
            for j in 0..3 {
                if px + i < self.width && py + j < self.height {
                    self.buffer.put_pixel(px + i, py + j, Rgb([200, 200, 255]));
                }
            }
        }
    }

    pub fn draw_photon_path(&mut self, path: &[(f64, f64)]) {
        for (x, y) in path {
            let px = *x as u32;
            let py = *y as u32;
            if px < self.width && py < self.height {
                // Green trace for photons
                self.buffer.put_pixel(px, py, Rgb([0, 255, 0]));
            }
        }
    }
    
    pub fn draw_absorption(&mut self, x: f64, y: f64) {
        let px = x as u32;
        let py = y as u32;
         if px < self.width && py < self.height {
            // Red X for absorption
            self.buffer.put_pixel(px, py, Rgb([255, 0, 0]));
         }
    }

    pub fn draw_carrier(&mut self, x: f64, y: f64, is_hole: bool) {
        let px = x as u32;
        let py = y as u32;
        if px < self.width && py < self.height {
            let color = if is_hole {
                Rgb([255, 255, 0]) // Yellow Holes
            } else {
                Rgb([0, 255, 255]) // Cyan Electrons
            };
            self.buffer.put_pixel(px, py, color);
        }
    }

    pub fn save(&self, filename: &str) {
        self.buffer.save(filename).expect("Failed to save image");
    }
}
