use physics_sim::genetic::Genome;

// Re-implement the fractal logic in Rust to match WGSL
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
    // Use same bounding box as shader: 10.0
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
        // p = abs(p + offset) - offset
        p = [
            (p[0] + offset[0]).abs() - offset[0],
            (p[1] + offset[1]).abs() - offset[1],
            (p[2] + offset[2]).abs() - offset[2]
        ];
        
        // Rotate
        p = rot_y(p, rot_angle);
        
        // Scale
        p = [p[0] * scale, p[1] * scale, p[2] * scale];
        
        // Subtract box to create channels
        // Note: We are subtracting a box of size 1.0 * scale
        // But since we scaled space by 'scale', a box of size 1.0 in this space corresponds to size 1/scale in original space.
        // To carve out a hole, we subtract.
        let box_dist = sd_box(p, [1.0, 1.0, 1.0]); 
        
        // Correct distance scaling
        let scale_factor = scale.powi(i + 1);
        
        // Intersection/Subtraction logic: d = max(d, -box_dist)
        // If we want to carve out the box from the current shape:
        d = max_float(d, -box_dist / scale_factor);
    }
    
    d 
}

fn main() {
    // The Best Genome from your run
    let best_genes: [f32; 16] = [
        0.7704699, 0.006599307, 0.14029002, 0.77480626, 0.14399093, 
        0.6627429, 0.73306084, 0.1451807, 0.106321275, 0.851491, 
        0.41654706, 0.87280214, 0.9913798, 0.35109216, 0.08714211, 0.8621229
    ];
    let genome = Genome { genes: best_genes };

    println!("Scanning 3D Geometry (Slice at Z=0)...");
    println!("Range: -15.0 to +15.0");
    println!("Legend: '#' = Solid, '.' = Near Surface, ' ' = Empty\n");

    // Scan grid
    let range = 15.0;
    let step = 0.5;
    
    let steps = ((range * 2.0) / step) as i32;
    
    for y_idx in 0..steps {
        let y = range - (y_idx as f32 * step); // Top to bottom
        for x_idx in 0..steps {
            let x = -range + (x_idx as f32 * step); // Left to right
            
            let dist = sd_fractal_channel([x, y, 0.0], &genome);
            
            if dist < 0.0 {
                print!("█"); // Inside Solid
            } else if dist < 0.2 {
                print!("#"); // Surface
            } else if dist < 1.0 {
                print!("."); // Aura
            } else {
                print!(" "); // Empty
            }
        }
        println!(); // Newline
    }
}