use std::collections::HashSet;

const GRID_SIZE: usize = 256;
const SIM_STEPS: usize = 20;

#[derive(Clone, Copy, Debug)]
struct Cell {
    op: u32,
    dir_a: u32,
    dir_b: u32,
}

fn main() {
    // The "Shader View" Genome (First 16 cells of the 8x8 Solved Genome)
    // Formed into a 4x4 grid.
    // Row 0 (4x4) = Left Half of Row 0 (8x8)
    // Row 1 (4x4) = Right Half of Row 0 (8x8)
    // Row 2 (4x4) = Left Half of Row 1 (8x8)
    // Row 3 (4x4) = Right Half of Row 1 (8x8)
    let genome_data: [(u32, u32, u32); 16] = [
        (0x0, 5, 5), (0x10, 4, 3), (0x10, 1, 2), (0x10, 6, 7),
        (0x1, 2, 3), (0x10, 3, 3), (0xB, 1, 6), (0x8, 1, 0),
        (0x0, 5, 1), (0x0, 7, 6), (0x0, 0, 7), (0x0, 2, 1),
        (0x5, 2, 1), (0x2, 1, 6), (0x0, 3, 7), (0x2, 6, 4),
    ];

    let mut grid = vec![Cell { op: 16, dir_a: 0, dir_b: 0 }; GRID_SIZE];

    // Place 4x4 Organism at (6,6) -> Indices 102..105, 118..121, 134..137, 150..153
    for i in 0..16 {
        let x = 6 + (i % 4);
        let y = 6 + (i / 4);
        let idx = y * 16 + x;
        let (op, da, db) = genome_data[i];
        grid[idx] = Cell { op, dir_a: da, dir_b: db };
    }

    println!("Analyzing 4x4 Hidden Quine (Shader View)...");
    
    let active_map = trace_active(&grid);
    
    println!("\n--- Active Cells (Vacuumed) ---");
    println!("   06       07       08       09");
    for y in 6..10 {
        let mut line = String::new();
        for x in 6..10 {
            let idx = y * 16 + x;
            if active_map[idx] {
                let c = grid[idx];
                line.push_str(&format!("[{:>2X} {}{}]  ", c.op, dir_arrow(c.dir_a), dir_arrow(c.dir_b)));
            } else {
                line.push_str("[.....]  ");
            }
        }
        println!("{:02}: {}", y, line);
    }
    
    let active_count = active_map.iter().filter(|&&b| b).count();
    println!("\nTotal Active Cells: {}", active_count);
    
    // Check Inputs (102..105)
    let inputs = [102, 103, 104, 105];
    print!("Inputs Used: ");
    for (i, &idx) in inputs.iter().enumerate() {
        if active_map[idx] { print!("{} ", i); }
    }
    println!();
    
    verify_quine(&grid, &genome_data, &active_map);
}

fn verify_quine(grid: &[Cell], genome_data: &[(u32, u32, u32); 16], active_map: &[bool]) {
    let mut total_matches = 0;
    
    println!("\nidx | Active? | Target | Output | Match?");
    println!("----|---------|--------|--------|-------");

    for i in 0..16 {
        let (target_op_raw, _, _) = genome_data[i];
        let target_op = if target_op_raw == 16 { 0 } else { target_op_raw };
        
        let x = 6 + (i % 4);
        let y = 6 + (i / 4);
        let grid_idx = y * 16 + x;
        let is_active = active_map[grid_idx];
        
        // Simulate Case i
        // Input: i (4 bits) injected at 102..105
        // Output: Read from 150..153
        
        let mut state_a = vec![0u32; GRID_SIZE];
        let mut state_b = vec![0u32; GRID_SIZE];
        
        for _ in 0..SIM_STEPS {
            for idx in 0..GRID_SIZE {
                let cell = grid[idx];
                let mut res = 0;
                
                if cell.op < 16 {
                    let na = get_neighbor_idx(idx as i32, cell.dir_a);
                    let nb = get_neighbor_idx(idx as i32, cell.dir_b);
                    let val_a = state_a[na];
                    let val_b = state_a[nb];
                    let lut_idx = (val_a << 1) | val_b;
                    res = (cell.op >> lut_idx) & 1;
                }
                
                // Force Inputs
                if idx >= 102 && idx <= 105 {
                    let bit_idx = idx - 102;
                    res = (i as u32 >> bit_idx) & 1;
                }
                
                state_b[idx] = res;
            }
            state_a = state_b.clone();
        }
        
        // Read Output
        let mut out_val = 0;
        for k in 0..4 {
            let idx = 150 + k;
            out_val |= state_a[idx] << k;
        }
        
        let match_str = if out_val == target_op { "YES" } else { "NO" };
        let active_str = if is_active { "YES" } else { " - " };
        println!(" {:2} |   {}   |    {:X}   |    {:X}   |  {}", i, active_str, target_op, out_val, match_str);
        
        if out_val == target_op { total_matches += 1; }
    }
    
    println!("----------------------------------------");
    println!("Total Matches: {} / 16", total_matches);
}

fn trace_active(grid: &[Cell]) -> Vec<bool> {
    let mut active_at_step = vec![HashSet::new(); SIM_STEPS + 1];
    let mut globally_active = vec![false; GRID_SIZE];
    
    // Seed with Outputs (150..153)
    let outputs = [150, 151, 152, 153];
    for &out_idx in &outputs {
        active_at_step[SIM_STEPS].insert(out_idx);
    }
    
    for t in (1..=SIM_STEPS).rev() {
        let current: Vec<usize> = active_at_step[t].iter().cloned().collect();
        for &idx in &current {
            globally_active[idx] = true;
            
            // Stop at inputs
            if idx >= 102 && idx <= 105 { continue; }
            
            let cell = grid[idx];
            if cell.op < 16 {
                let na = get_neighbor_idx(idx as i32, cell.dir_a);
                let nb = get_neighbor_idx(idx as i32, cell.dir_b);
                active_at_step[t-1].insert(na);
                active_at_step[t-1].insert(nb);
            }
        }
    }
    
    for set in active_at_step {
        for idx in set { globally_active[idx] = true; }
    }
    globally_active
}

fn get_neighbor_idx(idx: i32, dir: u32) -> usize {
    let x = idx % 16;
    let y = idx / 16;
    let mut dx = 0; let mut dy = 0;
    match dir {
        0 => { dx = 0; dy = -1; } // N
        1 => { dx = 1; dy = -1; } // NE
        2 => { dx = 1; dy = 0; }  // E
        3 => { dx = 1; dy = 1; }  // SE
        4 => { dx = 0; dy = 1; }  // S
        5 => { dx = -1; dy = 1; } // SW
        6 => { dx = -1; dy = 0; } // W
        7 => { dx = -1; dy = -1; } // NW
        _ => {}
    }
    let nx = (x + dx + 16) % 16;
    let ny = (y + dy + 16) % 16;
    (ny * 16 + nx) as usize
}

fn dir_arrow(dir: u32) -> char {
    match dir {
        0 => '↑', 1 => '↗', 2 => '→', 3 => '↘',
        4 => '↓', 5 => '↙', 6 => '←', 7 => '↖',
        _ => '?',
    }
}
