//! Component Test Harness
//!
//! Tests individual components (half adder, SR latch, etc.) on our 2D grid
//! using a CPU-based simulator for easy debugging and visualization.

use std::collections::HashMap;

// Operation constants
const OP_FALSE: u32 = 0b0000;
const OP_NOR: u32 = 0b0001;
const OP_XOR: u32 = 0b0110;
const OP_NAND: u32 = 0b0111;
const OP_AND: u32 = 0b1000;
const OP_B: u32 = 0b1010;
const OP_A: u32 = 0b1100;
const OP_OR: u32 = 0b1110;
const OP_TRUE: u32 = 0b1111;
const OP_VOID: u32 = 16;
const OP_JUMP: u32 = 17;

// Direction constants (8-way)
const DIR_N: u32 = 0;
const DIR_NE: u32 = 1;
const DIR_E: u32 = 2;
const DIR_SE: u32 = 3;
const DIR_S: u32 = 4;
const DIR_SW: u32 = 5;
const DIR_W: u32 = 6;
const DIR_NW: u32 = 7;

// Jump direction constants (4-way cardinal)
const JUMP_N: u32 = 0;
const JUMP_E: u32 = 1;
const JUMP_S: u32 = 2;
const JUMP_W: u32 = 3;

#[derive(Clone, Copy, Debug)]
struct Cell {
    op: u32,
    dir_a: u32,
    dir_b: u32,
    state: u32,
}

impl Cell {
    fn void() -> Self {
        Cell { op: OP_VOID, dir_a: 0, dir_b: 0, state: 0 }
    }

    fn new(op: u32, dir_a: u32, dir_b: u32) -> Self {
        Cell { op, dir_a, dir_b, state: 0 }
    }

    fn jump(direction: u32, distance: u32) -> Self {
        Cell { op: OP_JUMP, dir_a: direction, dir_b: distance, state: 0 }
    }
}

struct Grid {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl Grid {
    fn new(width: usize, height: usize) -> Self {
        Grid {
            width,
            height,
            cells: vec![Cell::void(); width * height],
        }
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn get(&self, x: usize, y: usize) -> &Cell {
        &self.cells[self.idx(x, y)]
    }

    fn get_mut(&mut self, x: usize, y: usize) -> &mut Cell {
        let idx = self.idx(x, y);
        &mut self.cells[idx]
    }

    fn set(&mut self, x: usize, y: usize, cell: Cell) {
        let idx = self.idx(x, y);
        self.cells[idx] = cell;
    }

    fn set_state(&mut self, x: usize, y: usize, state: u32) {
        let idx = self.idx(x, y);
        self.cells[idx].state = state;
    }

    fn get_state(&self, x: usize, y: usize) -> u32 {
        self.cells[self.idx(x, y)].state
    }

    /// Get neighbor coordinates with wrapping
    fn neighbor(&self, x: usize, y: usize, dir: u32) -> (usize, usize) {
        let (dx, dy) = match dir {
            DIR_N => (0i32, -1i32),
            DIR_NE => (1, -1),
            DIR_E => (1, 0),
            DIR_SE => (1, 1),
            DIR_S => (0, 1),
            DIR_SW => (-1, 1),
            DIR_W => (-1, 0),
            DIR_NW => (-1, -1),
            _ => (0, 0),
        };
        let nx = ((x as i32 + dx).rem_euclid(self.width as i32)) as usize;
        let ny = ((y as i32 + dy).rem_euclid(self.height as i32)) as usize;
        (nx, ny)
    }

    /// Get jump target coordinates with wrapping
    fn jump_target(&self, x: usize, y: usize, direction: u32, distance: u32) -> (usize, usize) {
        let dist = distance as i32;
        let (dx, dy) = match direction {
            JUMP_N => (0, -dist),
            JUMP_E => (dist, 0),
            JUMP_S => (0, dist),
            JUMP_W => (-dist, 0),
            _ => (0, 0),
        };
        let nx = ((x as i32 + dx).rem_euclid(self.width as i32)) as usize;
        let ny = ((y as i32 + dy).rem_euclid(self.height as i32)) as usize;
        (nx, ny)
    }

    /// Run one simulation step, preserving forced inputs
    fn step_with_inputs(&mut self, inputs: &[(usize, usize, u32)]) {
        let mut new_states = vec![0u32; self.cells.len()];

        for y in 0..self.height {
            for x in 0..self.width {
                let cell = self.get(x, y);

                let new_state = if cell.op < 16 {
                    // Logic gate: read two neighbors, apply LUT
                    let (ax, ay) = self.neighbor(x, y, cell.dir_a);
                    let (bx, by) = self.neighbor(x, y, cell.dir_b);
                    let val_a = self.get_state(ax, ay);
                    let val_b = self.get_state(bx, by);
                    let lut_idx = (val_a << 1) | val_b;
                    (cell.op >> lut_idx) & 1
                } else if cell.op == OP_JUMP {
                    // Jump wire: read from distant cell
                    let (tx, ty) = self.jump_target(x, y, cell.dir_a, cell.dir_b);
                    self.get_state(tx, ty)
                } else {
                    // VOID or unknown: output 0
                    0
                };

                new_states[self.idx(x, y)] = new_state;
            }
        }

        // Apply new states
        for (i, &state) in new_states.iter().enumerate() {
            self.cells[i].state = state;
        }

        // Re-apply forced inputs (they persist each step)
        for &(x, y, val) in inputs {
            self.set_state(x, y, val);
        }
    }

    /// Run one simulation step
    fn step(&mut self) {
        self.step_with_inputs(&[]);
    }

    /// Run multiple steps with forced inputs
    fn run_with_inputs(&mut self, steps: usize, inputs: &[(usize, usize, u32)]) {
        // Set initial input values
        for &(x, y, val) in inputs {
            self.set_state(x, y, val);
        }
        for _ in 0..steps {
            self.step_with_inputs(inputs);
        }
    }

    /// Run multiple steps
    fn run(&mut self, steps: usize) {
        for _ in 0..steps {
            self.step();
        }
    }

    /// Print grid state
    fn print(&self) {
        println!("Grid state ({}x{}):", self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = self.get(x, y);
                let ch = if cell.op == OP_VOID {
                    '·'
                } else {
                    if cell.state == 1 { '█' } else { '░' }
                };
                print!("{}", ch);
            }
            println!();
        }
        println!();
    }

    /// Print grid with op codes
    fn print_ops(&self) {
        println!("Grid ops:");
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = self.get(x, y);
                let s = match cell.op {
                    OP_VOID => " · ".to_string(),
                    OP_JUMP => format!("J{}{}",
                        match cell.dir_a { 0 => 'N', 1 => 'E', 2 => 'S', _ => 'W' },
                        cell.dir_b),
                    op if op < 16 => format!("{:X}{}{}", op,
                        match cell.dir_a { 0 => '↑', 1 => '↗', 2 => '→', 3 => '↘', 4 => '↓', 5 => '↙', 6 => '←', _ => '↖' },
                        match cell.dir_b { 0 => '↑', 1 => '↗', 2 => '→', 3 => '↘', 4 => '↓', 5 => '↙', 6 => '←', _ => '↖' }),
                    _ => " ? ".to_string(),
                };
                print!("{} ", s);
            }
            println!();
        }
        println!();
    }
}

// ============================================================================
// COMPONENT BUILDERS
// ============================================================================

/// Build a half adder at position (x, y)
/// Inputs come from (x, y-1) and (x+1, y-1)
/// Outputs: Sum at (x, y), Carry at (x+1, y)
fn build_half_adder(grid: &mut Grid, x: usize, y: usize) {
    // XOR for Sum: reads North (A) and NorthEast (B)
    grid.set(x, y, Cell::new(OP_XOR, DIR_N, DIR_NE));
    // AND for Carry: reads NorthWest (A) and North (B)
    grid.set(x + 1, y, Cell::new(OP_AND, DIR_NW, DIR_N));
}

/// Build an SR latch at position (x, y)
/// Uses cross-coupled NOR gates
/// S input at (x, y-1), R input at (x+1, y-1)
/// Q output at (x, y+1), Q' output at (x+1, y+1)
fn build_sr_latch(grid: &mut Grid, x: usize, y: usize) {
    // Top row: NOR gates with cross-coupling
    // Left NOR: inputs from S (North) and Q' (East)
    grid.set(x, y, Cell::new(OP_NOR, DIR_N, DIR_E));
    // Right NOR: inputs from R (North) and Q (West)
    grid.set(x + 1, y, Cell::new(OP_NOR, DIR_N, DIR_W));

    // Bottom row: Output wires
    grid.set(x, y + 1, Cell::new(OP_A, DIR_N, DIR_N));      // Q
    grid.set(x + 1, y + 1, Cell::new(OP_A, DIR_N, DIR_N));  // Q'
}

/// Build a 4-bit shift register (horizontal, shifts right)
/// Input at (x-1, y), outputs ripple through (x, y) to (x+3, y)
fn build_shift_register_4(grid: &mut Grid, x: usize, y: usize) {
    // Each cell passes its west neighbor's state
    for i in 0..4 {
        grid.set(x + i, y, Cell::new(OP_A, DIR_W, DIR_W));
    }
}

/// Build a ring oscillator (3 inverters in a loop)
fn build_ring_oscillator(grid: &mut Grid, x: usize, y: usize) {
    // NOT_A (op=3) inverts input A
    // Form a triangle or line that loops back
    grid.set(x, y, Cell::new(0b0011, DIR_E, DIR_E));      // NOT of East
    grid.set(x + 1, y, Cell::new(0b0011, DIR_E, DIR_E));  // NOT of East
    grid.set(x + 2, y, Cell::new(0b0011, DIR_W, DIR_W));  // NOT of West (wraps around via jump)

    // Need to close the loop - use a jump wire
    // Cell at x+2 reads from x, creating odd-length inverter chain
    grid.set(x + 2, y, Cell::jump(JUMP_W, 2));  // Jump 2 West to read from x
}

/// Test jump wires by creating a long-distance signal path
fn build_jump_test(grid: &mut Grid, x: usize, y: usize) {
    // Source signal
    grid.set(x, y, Cell::new(OP_TRUE, 0, 0));  // Always 1

    // Jump 3 cells East
    grid.set(x + 3, y, Cell::jump(JUMP_W, 3));

    // Jump 2 cells South
    grid.set(x + 3, y + 2, Cell::jump(JUMP_N, 2));

    // Continue with regular wire
    grid.set(x + 3, y + 3, Cell::new(OP_A, DIR_N, DIR_N));
}

// ============================================================================
// TESTS
// ============================================================================

fn test_half_adder() {
    println!("=== HALF ADDER TEST ===\n");

    // Minimal grid for half adder
    // Layout:
    //   Row 0: [void] [A input] [B input]
    //   Row 1: [void] [XOR=Sum] [AND=Carry]
    //
    // XOR at (1,1) reads North (A) and East (B)
    // AND at (2,1) reads North (B) and West (XOR output, but we want A)
    //
    // Actually simpler: Both gates read the same two inputs
    // XOR at (1,1): reads N=(1,0)=A, NE=(2,0)=B
    // AND at (2,1): reads NW=(1,0)=A, N=(2,0)=B

    let mut grid = Grid::new(4, 3);

    // Input positions (row 0): A at (1,0), B at (2,0)
    // These are just VOID cells where we inject state

    // Half adder gates (row 1)
    grid.set(1, 1, Cell::new(OP_XOR, DIR_N, DIR_NE));   // Sum = A XOR B
    grid.set(2, 1, Cell::new(OP_AND, DIR_NW, DIR_N));   // Carry = A AND B

    grid.print_ops();

    println!("Testing all input combinations:");
    println!("A B | Sum Carry");
    println!("----+---------");

    let mut all_pass = true;
    for a in 0..=1u32 {
        for b in 0..=1u32 {
            // Reset grid
            for cell in &mut grid.cells {
                cell.state = 0;
            }

            // Run with forced inputs at A=(1,0), B=(2,0)
            let inputs = vec![(1, 0, a), (2, 0, b)];
            grid.run_with_inputs(2, &inputs);

            // Read outputs
            let sum = grid.get_state(1, 1);
            let carry = grid.get_state(2, 1);

            let expected_sum = a ^ b;
            let expected_carry = a & b;

            let ok = sum == expected_sum && carry == expected_carry;
            if !ok { all_pass = false; }
            println!("{} {} |  {}    {}    {}", a, b, sum, carry,
                     if ok { "✓" } else { "✗" });
        }
    }
    println!("Half adder: {}\n", if all_pass { "PASS" } else { "FAIL" });
}

fn test_sr_latch() {
    println!("=== SR LATCH TEST ===\n");

    // SR Latch using cross-coupled NOR gates
    // Standard SR NOR latch: Q = NOR(R, Q'), Q' = NOR(S, Q)
    //
    // Layout:
    //   Row 0: [R input]  [S input]
    //   Row 1: [NOR Q]    [NOR Q']
    //
    // Left NOR at (0,1): reads N=(0,0)=R, E=(1,1)=Q'  → output is Q
    // Right NOR at (1,1): reads W=(0,1)=Q, N=(1,0)=S   → output is Q'

    let mut grid = Grid::new(3, 3);

    // SR latch NOR gates
    grid.set(0, 1, Cell::new(OP_NOR, DIR_N, DIR_E));   // Q = NOR(R, Q')
    grid.set(1, 1, Cell::new(OP_NOR, DIR_W, DIR_N));   // Q' = NOR(Q, S)

    grid.print_ops();

    println!("Testing SR Latch behavior:");
    println!("(SR Latch: R at (0,0), S at (1,0))\n");

    // Reset grid state
    for cell in &mut grid.cells {
        cell.state = 0;
    }

    // Test sequence with expected behavior
    // Inputs: R at (0,0), S at (1,0)
    // SR NOR truth table:
    //   S=0,R=0: Hold (Q unchanged)
    //   S=0,R=1: Reset (Q=0, Q'=1)
    //   S=1,R=0: Set (Q=1, Q'=0)
    //   S=1,R=1: Invalid (both 0)
    let tests = [
        ("Reset (R=1, S=0)", 1u32, 0u32, 0u32, 1u32),   // Q=0, Q'=1
        ("Hold  (R=0, S=0)", 0, 0, 0, 1),               // Q=0, Q'=1 (remembers)
        ("Set   (R=0, S=1)", 0, 1, 1, 0),               // Q=1, Q'=0
        ("Hold  (R=0, S=0)", 0, 0, 1, 0),               // Q=1, Q'=0 (remembers)
        ("Reset (R=1, S=0)", 1, 0, 0, 1),               // Q=0, Q'=1
    ];

    let mut all_pass = true;
    for (name, r, s, expected_q, expected_q_bar) in tests {
        // R at (0,0), S at (1,0)
        let inputs = vec![(0, 0, r), (1, 0, s)];

        // Run several steps for latch to settle
        for _ in 0..10 {
            grid.step_with_inputs(&inputs);
        }

        let q = grid.get_state(0, 1);
        let q_bar = grid.get_state(1, 1);

        let ok = q == expected_q && q_bar == expected_q_bar;
        if !ok { all_pass = false; }
        println!("{}: R={} S={} → Q={} Q'={} (expected Q={} Q'={}) {}",
                 name, r, s, q, q_bar, expected_q, expected_q_bar,
                 if ok { "✓" } else { "✗" });
    }
    println!("SR Latch: {}\n", if all_pass { "PASS" } else { "FAIL" });
}

fn test_shift_register() {
    println!("=== 4-BIT SHIFT REGISTER TEST ===\n");

    // Shift register: chain of cells reading from West neighbor
    // Layout: Input → [bit0] → [bit1] → [bit2] → [bit3]
    //
    // Each cell passes its West neighbor's value to itself
    // Using OP_A with DIR_W means: output = input_A = west_neighbor

    let mut grid = Grid::new(6, 1);

    // 4 register cells at positions 1-4 (position 0 is input)
    for i in 1..=4 {
        grid.set(i, 0, Cell::new(OP_A, DIR_W, DIR_W));
    }

    grid.print_ops();

    println!("Shifting a '1' bit through (input at position 0):");

    // Reset
    for cell in &mut grid.cells {
        cell.state = 0;
    }

    let mut all_pass = true;
    // Shift sequence: inject 1, then 0s
    let input_sequence = [1u32, 0, 0, 0, 0, 0, 0, 0];
    let expected_outputs = [
        [0, 0, 0, 0],  // step 0: nothing yet
        [1, 0, 0, 0],  // step 1: 1 enters
        [0, 1, 0, 0],  // step 2: 1 shifts
        [0, 0, 1, 0],  // step 3
        [0, 0, 0, 1],  // step 4
        [0, 0, 0, 0],  // step 5: 1 exits
        [0, 0, 0, 0],  // step 6
        [0, 0, 0, 0],  // step 7
    ];

    for step in 0..8 {
        let bits: Vec<u32> = (1..=4).map(|x| grid.get_state(x, 0)).collect();
        let expected: Vec<u32> = expected_outputs[step].to_vec();
        let ok = bits == expected;
        if !ok { all_pass = false; }
        println!("Step {}: {:?} (expected {:?}) {}",
                 step, bits, expected, if ok { "✓" } else { "✗" });

        // Set input for next step
        grid.set_state(0, 0, input_sequence[step]);
        grid.step();
    }
    println!("Shift register: {}\n", if all_pass { "PASS" } else { "FAIL" });
}

fn test_jump_wires() {
    println!("=== JUMP WIRE TEST ===\n");

    // Test jump wires carrying signals over distance
    // Layout: Signal source at (0,0), jump wires at various distances
    //
    //  (0,0) --jump3E--> (3,0) --jump2S--> (3,2) --jump4W--> wraps to (grid edge)
    //
    // Using forced input at source position

    let mut grid = Grid::new(8, 4);

    // Jump chain:
    // (0,0) = source (forced input)
    // (3,0) = jump 3 west to read (0,0)
    grid.set(3, 0, Cell::jump(JUMP_W, 3));
    // (3,2) = jump 2 north to read (3,0)
    grid.set(3, 2, Cell::jump(JUMP_N, 2));
    // (7,2) = jump 4 west to read (3,2)
    grid.set(7, 2, Cell::jump(JUMP_W, 4));

    grid.print_ops();

    // Reset
    for cell in &mut grid.cells {
        cell.state = 0;
    }

    println!("Signal propagation with forced input=1 at (0,0):");
    let inputs = vec![(0usize, 0usize, 1u32)];

    // Expected: signal takes 1 step per hop
    // Step 0: source=1, others=0
    // Step 1: (3,0)=1
    // Step 2: (3,2)=1
    // Step 3: (7,2)=1

    let mut all_pass = true;
    let expected = [
        (0, 0, 0),  // step 0: signal at source only
        (1, 0, 0),  // step 1: jump1 receives
        (1, 1, 0),  // step 2: jump2 receives
        (1, 1, 1),  // step 3: jump3 receives
    ];

    for step in 0..4 {
        grid.step_with_inputs(&inputs);

        let j1 = grid.get_state(3, 0);
        let j2 = grid.get_state(3, 2);
        let j3 = grid.get_state(7, 2);

        let (e1, e2, e3) = expected[step];
        let ok = j1 == e1 && j2 == e2 && j3 == e3;
        if !ok { all_pass = false; }

        println!("Step {}: Jump3W(3,0)={} Jump2N(3,2)={} Jump4W(7,2)={} (expected {},{},{}) {}",
                 step, j1, j2, j3, e1, e2, e3, if ok { "✓" } else { "✗" });
    }
    println!("Jump wires: {}\n", if all_pass { "PASS" } else { "FAIL" });
}

fn test_full_adder() {
    println!("=== FULL ADDER TEST ===\n");

    // Full adder: Sum = A XOR B XOR Cin, Cout = (A AND B) OR ((A XOR B) AND Cin)
    //
    // Clean layout using clear routing paths:
    // - Column 0: S1 (A XOR B) path
    // - Column 1: C1 (A AND B) path
    // - Column 3: Cin path
    // - Use diagonal reading and jumps to connect

    let mut grid = Grid::new(6, 8);

    // === Row 0: Inputs ===
    // A at (0,0), B at (1,0), Cin at (3,0)

    // === Row 1: First half adder ===
    grid.set(0, 1, Cell::new(OP_XOR, DIR_N, DIR_NE));  // S1 = A XOR B: reads A(0,0), B(1,0)
    grid.set(1, 1, Cell::new(OP_AND, DIR_NW, DIR_N));  // C1 = A AND B: reads A(0,0), B(1,0)
    grid.set(3, 1, Cell::new(OP_A, DIR_N, DIR_N));     // Cin wire: reads Cin(3,0)

    // === Row 2: Continue routing ===
    grid.set(0, 2, Cell::new(OP_A, DIR_N, DIR_N));     // S1 continues
    grid.set(1, 2, Cell::new(OP_A, DIR_N, DIR_N));     // C1 continues
    grid.set(3, 2, Cell::new(OP_A, DIR_N, DIR_N));     // Cin continues
    grid.set(2, 2, Cell::new(OP_A, DIR_E, DIR_E));     // Cin branch west: reads (3,2)

    // === Row 3: Sum calculation ===
    // Sum = S1 XOR Cin
    // Put Sum at (1,3): reads NW=(0,2)=S1, NE=(2,2)=Cin_branch
    grid.set(1, 3, Cell::new(OP_XOR, DIR_NW, DIR_NE)); // Sum
    grid.set(0, 3, Cell::new(OP_A, DIR_N, DIR_N));     // S1 continues (for C2)
    grid.set(3, 3, Cell::new(OP_A, DIR_N, DIR_N));     // Cin continues (for C2)
    grid.set(2, 3, Cell::new(OP_A, DIR_E, DIR_E));     // Cin branch west: reads (3,3)

    // === Row 4: Continue for C2 ===
    grid.set(0, 4, Cell::new(OP_A, DIR_N, DIR_N));     // S1 continues
    grid.set(2, 4, Cell::new(OP_A, DIR_N, DIR_N));     // Cin continues

    // === Row 5: C2 calculation ===
    // C2 = S1 AND Cin
    // Put C2 at (1,5): reads NW=(0,4)=S1, NE=(2,4)=Cin
    grid.set(1, 5, Cell::new(OP_AND, DIR_NW, DIR_NE)); // C2

    // === C1 routing to row 6 ===
    // C1 is at (1,2). Use jump to bring it down.
    grid.set(4, 2, Cell::jump(JUMP_W, 3));             // Jump reads (1,2)=C1
    grid.set(4, 3, Cell::new(OP_A, DIR_N, DIR_N));     // C1 route
    grid.set(4, 4, Cell::new(OP_A, DIR_N, DIR_N));     // C1 route
    grid.set(4, 5, Cell::new(OP_A, DIR_N, DIR_N));     // C1 route

    // === Row 6: Cout calculation ===
    // Cout = C1 OR C2
    // Wire C2 and C1 to adjacent positions for OR
    grid.set(2, 6, Cell::new(OP_A, DIR_NW, DIR_NW));   // C2 wire: reads (1,5)=C2
    grid.set(4, 6, Cell::new(OP_A, DIR_N, DIR_N));     // C1 wire: reads (4,5)

    // OR at (3,6): reads W=(2,6)=C2_wire, E=(4,6)=C1_wire
    grid.set(3, 6, Cell::new(OP_OR, DIR_W, DIR_E));    // Cout

    grid.print_ops();

    println!("Testing full adder (A + B + Cin):");
    println!("A B Cin | Sum Cout | Expected");
    println!("--------+----------+---------");

    let mut all_pass = true;
    for a in 0..=1u32 {
        for b in 0..=1u32 {
            for cin in 0..=1u32 {
                // Reset
                for cell in &mut grid.cells {
                    cell.state = 0;
                }

                // Inputs: A(0,0), B(1,0), Cin(3,0)
                let inputs = vec![(0, 0, a), (1, 0, b), (3, 0, cin)];
                grid.run_with_inputs(8, &inputs);

                let sum = grid.get_state(1, 3);
                let cout = grid.get_state(3, 6);

                let expected_total = a + b + cin;
                let expected_sum = expected_total & 1;
                let expected_cout = (expected_total >> 1) & 1;

                let ok = sum == expected_sum && cout == expected_cout;
                if !ok { all_pass = false; }
                println!("{} {}  {}  |  {}    {}   |  {}    {}    {}",
                         a, b, cin, sum, cout, expected_sum, expected_cout,
                         if ok { "✓" } else { "✗" });
            }
        }
    }
    println!("Full adder: {}\n", if all_pass { "PASS" } else { "FAIL" });
}

fn main() {
    println!("╔═══════════════════════════════════════╗");
    println!("║   COMPONENT TEST HARNESS              ║");
    println!("║   Testing logic components on 2D grid ║");
    println!("╚═══════════════════════════════════════╝\n");

    test_half_adder();
    test_sr_latch();
    test_shift_register();
    test_jump_wires();
    test_full_adder();

    println!("All tests complete!");
}
