use physics_sim::genetic::Genome;
use image::{ImageBuffer, Rgb};

// Re-implement the fractal logic (Same as verify_geometry.rs)
fn rot_y(p: [f32; 3], a: f32) -> [f32; 3] {
    let c = a.cos();
    let s = a.sin();
    [
        c * p[0] + s * p[2],
        p[1],
        -s * p[0] + c * p[2]
    ]
}

fn length(p: [f32; 3]) -> f32 {
    (p[0]*p[0] + p[1]*p[1] + p[2]*p[2]).sqrt()
}

fn max_vec3(a: [f32; 3], b: f32) -> [f32; 3] {
    [a[0].max(b), a[1].max(b), a[2].max(b)]
}

fn max_float(a: f32, b: f32) -> f32 {
    a.max(b)
}

fn sd_box(p: [f32; 3], b: [f32; 3]) -> f32 {
    let q = [
        p[0].abs() - b[0],
        p[1].abs() - b[1],
        p[2].abs() - b[2]
    ];
    let max_q = max_vec3(q, 0.0);
    length(max_q) + q[0].max(q[1]).max(q[2]).min(0.0)
}

fn sd_fractal_channel(pos: [f32; 3], genes: &Genome) -> f32 {
    let mut p = pos;
    let mut d = sd_box(p, [10.0, 10.0, 10.0]); 

    let iterations = 8;
    let scale = genes.genes[0] * 2.0 + 1.0;
    let offset = [
        genes.genes[1] * 5.0,
        genes.genes[2] * 5.0,
        genes.genes[3] * 5.0
    ];
    let rot_angle = genes.genes[4] * 3.14159 * 2.0;

    for i in 0..iterations {
        p = [
            (p[0] + offset[0]).abs() - offset[0],
            (p[1] + offset[1]).abs() - offset[1],
            (p[2] + offset[2]).abs() - offset[2]
        ];
        
        p = rot_y(p, rot_angle);
        p = [p[0] * scale, p[1] * scale, p[2] * scale];
        
        let box_dist = sd_box(p, [1.0, 1.0, 1.0]); 
        let scale_factor = scale.powi(i + 1);
        d = max_float(d, -box_dist / scale_factor);
    }
    
    d 
}

fn main() {
    let best_genes: [f32; 16] = [
        0.69660175, 0.31791842, 0.5483368, 0.94916594, 0.376301, 
        0.67482615, 0.5715449, 0.48322672, 0.528438, 0.5595225, 
        0.25746667, 0.14563656, 0.83600473, 0.2548437, 0.37280405, 0.6571505
    ];
    let genome = Genome { genes: best_genes };

    let width = 1024;
    let height = 1024;
    let range = 15.0; 

    println!("Generating 2D Slice ({}x{})...", width, height);

    let mut img = ImageBuffer::new(width, height);

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        // Map pixel to world coordinates
        let world_x = (x as f32 / width as f32) * 2.0 * range - range;
        let world_y = (y as f32 / height as f32) * 2.0 * range - range;
        // world_y is inverted in image space usually, let's flip it
        let world_y = -((y as f32 / height as f32) * 2.0 * range - range);

        let dist = sd_fractal_channel([world_x, world_y, 0.0], &genome);

        if dist < 0.0 {
            // Solid (Inside)
            *pixel = Rgb([20, 20, 40]); // Dark Blue
        } else {
            // Empty (Outside)
            // Color based on distance (Distance Field Visualization)
            let d_norm = (dist / 5.0).clamp(0.0, 1.0);
            let val = (d_norm * 255.0) as u8;
            *pixel = Rgb([255 - val, 255 - val / 2, 255]); // White -> Orange gradient
        }
    }

    img.save("genome_slice.png").unwrap();
    println!("Saved 'genome_slice.png'");
}
