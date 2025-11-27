use std::collections::HashSet;

// 16x16 Grid
const GRID_DIM: i32 = 16;
const GRID_SIZE: usize = 256;
const SIM_STEPS: usize = 20;

// I/O from Log
const INPUTS: [usize; 8] = [211, 225, 240, 5, 13, 199, 1, 148];
const OUTPUTS: [usize; 8] = [134, 11, 198, 227, 145, 40, 127, 191];

#[derive(Clone, Copy, Debug)]
struct Cell {
    op: u32,
    dir_a: u32,
    dir_b: u32,
}

fn main() {
    // The Solved Genome (First 16 cells / Top 2 rows of 8x8 print)
    // Derived from the log output:
    // [10 7 7] [6 3 2] [0 6 7] [0 7 4] [7 5 3] [7 2 6] [A 7 4] [E 6 6] 
    // [1 7 4] [7 0 2] [6 6 0] [0 4 0] [3 1 7] [C 6 2] [7 3 2] [0 1 0]
    let genome_data = [
        (0x10, 7, 7), (0x6, 3, 2), (0x0, 6, 7), (0x0, 7, 4), (0x7, 5, 3), (0x7, 2, 6), (0xA, 7, 4), (0xE, 6, 6),
        (0x1, 7, 4), (0x7, 0, 2), (0x6, 6, 0), (0x0, 4, 0), (0x3, 1, 7), (0xC, 6, 2), (0x7, 3, 2), (0x0, 1, 0)
    ];

    let mut grid = vec![Cell { op: 16, dir_a: 0, dir_b: 0 }; GRID_SIZE];

    // Reconstruct the 4x4 grid `organism` from `genome_data`
    let organism_cells: Vec<Cell> = genome_data.iter().map(|&(op, da, db)| Cell { op, dir_a: da, dir_b: db }).collect();
    
    // Place into 16x16 grid at (6,6)
    for i in 0..16 {
        let x = 6 + (i % 4);
        let y = 6 + (i / 4);
        let grid_idx = y * 16 + x;
        grid[grid_idx] = organism_cells[i];
    }
    
    println!("Analyzing 4x4 Organism (Active Trace)...");
    
    // Run Trace
    let active_map = trace_active(&grid);
    
    // Print Result
    println!("\n--- Active Cells (Vacuumed) ---");
    println!("   04       05       06       07       08       09       10       11");
    
    for y in 4..12 {
        let mut line = String::new();
        for x in 4..12 {
            let idx = y * 16 + x;
            if active_map[idx] {
                let c = grid[idx];
                let da = dir_arrow(c.dir_a);
                let db = dir_arrow(c.dir_b);
                // Op is 0-16. 16=0x10.
                // Format: [Op DA DB]
                // Fixed width 9 chars: "[10 ↑↑]  "
                line.push_str(&format!("[{:>2X} {}{}]  ", c.op, da, db));
            } else if x >= 6 && x < 10 && y >= 6 && y < 10 {
                line.push_str("[.....]  "); // Inactive part of organism
            } else {
                line.push_str(" . . .   "); // Empty space
            }
        }
        println!("{:02}: {}", y, line);
    }
    
    // Stats
    let active_count = active_map.iter().filter(|&&b| b).count();
    // Corrected format string: single braces
    println!("\nTotal Active Cells: {}", active_count);
    
    // List inputs used
    println!("Inputs Used:");
    for (k, &idx) in INPUTS.iter().enumerate() {
        if active_map[idx] {
            println!("  Input {} at ({}, {}) is ACTIVE", k, idx%16, idx/16);
        }
    }
    
    println!("Outputs Used:");
    for (k, &idx) in OUTPUTS.iter().enumerate() {
        if active_map[idx] {
            println!("  Output {} at ({}, {}) is DRIVEN", k, idx%16, idx/16);
        }
    }
    
    println!("\n--- Verification (CPU Simulation) ---");
    verify_quine(&grid, &organism_cells);
}

fn verify_quine(grid: &[Cell], organism: &[Cell]) {
    let mut total_matches = 0;
    
    println!("idx | Target (Op) | Output (Op) | Match?");
    println!("----|-------------|-------------|-------");

    for case_id in 0..16 {
        let target_op = organism[case_id].op;
        
        // Run Simulation
        let mut state_a = vec![0u32; GRID_SIZE];
        let mut state_b = vec![0u32; GRID_SIZE];
        
        for _step in 0..SIM_STEPS {
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
                
                // Force Input
                // Shader: if (y == 6u && x >= 6u && x < 10u)
                // Indices: 102, 103, 104, 105
                if idx >= 102 && idx <= 105 {
                    let bit_idx = idx - 102;
                    res = (case_id as u32 >> bit_idx) & 1;
                }
                
                state_b[idx] = res;
            }
            state_a = state_b.clone();
        }
        
        // Read Output
        // Indices: 150, 151, 152, 153
        let mut out_val = 0;
        for k in 0..4 {
            let idx = 150 + k;
            out_val |= state_a[idx] << k;
        }
        
        let match_str = if out_val == target_op { "YES" } else { "NO" };
        println!(" {:2} |      {:X}      |      {:X}      |  {}", case_id, target_op, out_val, match_str);
        
        if out_val == target_op { total_matches += 1; }
    }
    
    println!("----------------------------------------");
    println!("Total Matches: {} / 16", total_matches);
}

fn trace_active(grid: &[Cell]) -> Vec<bool> {
    let mut active_at_step = vec![HashSet::new(); SIM_STEPS + 1];
    let mut globally_active = vec![false; GRID_SIZE];
    
    // Seed with Output Ports (Relative to Organism)
    // Indices corresponding to (6,9)..(9,9) -> 150..153
    let outputs = [150, 151, 152, 153];
    
    for &out_idx in &outputs {
        active_at_step[SIM_STEPS].insert(out_idx);
    }
    
    // Propagate
    for t in (1..=SIM_STEPS).rev() {
        // Clone the set for current step to avoid borrow conflict
        let current_active_cells: Vec<usize> = active_at_step[t].iter().cloned().collect();
        
        for &idx in &current_active_cells {
            globally_active[idx] = true;
            
            let cell = grid[idx];
            
            // Check if Input Port (Indices 102..105)
            let is_input_port = idx >= 102 && idx <= 105;
            
            if is_input_port {
                continue; // Source
            }
            
            if cell.op < 16 {
                let na = get_neighbor_idx(idx as i32, cell.dir_a);
                let nb = get_neighbor_idx(idx as i32, cell.dir_b);
                active_at_step[t-1].insert(na);
                active_at_step[t-1].insert(nb);
            }
        }
    }
    
    // Union
    for set in active_at_step {
        for idx in set {
            globally_active[idx] = true;
        }
    }
    
    globally_active
}

fn get_neighbor_idx(idx: i32, dir: u32) -> usize {
    let x = idx % 16;
    let y = idx / 16;
    let mut dx = 0; let mut dy = 0;
    match dir {
        0 => { dx = 0; dy = -1; }
        1 => { dx = 1; dy = -1; }
        2 => { dx = 1; dy = 0; }
        3 => { dx = 1; dy = 1; }
        4 => { dx = 0; dy = 1; }
        5 => { dx = -1; dy = 1; }
        6 => { dx = -1; dy = 0; }
        7 => { dx = -1; dy = -1; }
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
