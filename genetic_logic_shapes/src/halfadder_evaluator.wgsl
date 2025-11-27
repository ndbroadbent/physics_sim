// Half Adder Evaluator
//
// Input: 2 bits (A, B)
// Output: 2 bits (Sum = A XOR B, Carry = A AND B)
// 4 test cases: all combinations

struct Cell {
    op: u32,
    dir_a: u32,
    dir_b: u32,
    flags: u32,
}

@group(0) @binding(0) var<storage, read> genomes: array<Cell>;
@group(0) @binding(1) var<storage, read_write> results: array<u32>;

const GRID_DIM: u32 = 4u;      // Tiny 4x4 grid
const GENOME_SIZE: u32 = 16u;
const SIM_STEPS: u32 = 10u;
const TEST_CASES: u32 = 4u;

// Operation constants
const OP_VOID: u32 = 16u;
const OP_JUMP: u32 = 17u;

// Jump direction constants
const JUMP_N: u32 = 0u;
const JUMP_E: u32 = 1u;
const JUMP_S: u32 = 2u;
const JUMP_W: u32 = 3u;

struct SimCell {
    op: u32,
    dir_a: u32,
    dir_b: u32,
    flags: u32,
    state: u32,
}

var<workgroup> grid_a: array<SimCell, 16>;
var<workgroup> grid_b: array<u32, 16>;

// Fixed I/O positions
// Inputs at top row: cells 0,1 (A, B)
// Outputs at bottom row: cells 12,13 (Sum, Carry)
const INPUT_A: u32 = 0u;    // A at (0,0)
const INPUT_B: u32 = 1u;    // B at (1,0)
const OUTPUT_SUM: u32 = 12u;   // Sum at (0,3)
const OUTPUT_CARRY: u32 = 13u; // Carry at (1,3)

fn get_neighbor_idx(idx: u32, dir: u32) -> u32 {
    let x = i32(idx % GRID_DIM);
    let y = i32(idx / GRID_DIM);
    var dx = 0i;
    var dy = 0i;

    // Direction encoding: 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW
    if (dir == 0u || dir == 1u || dir == 7u) { dy = -1i; }
    if (dir == 3u || dir == 4u || dir == 5u) { dy = 1i; }
    if (dir == 1u || dir == 2u || dir == 3u) { dx = 1i; }
    if (dir == 5u || dir == 6u || dir == 7u) { dx = -1i; }

    let nx = (x + dx + i32(GRID_DIM)) % i32(GRID_DIM);
    let ny = (y + dy + i32(GRID_DIM)) % i32(GRID_DIM);
    return u32(ny) * GRID_DIM + u32(nx);
}

fn get_jump_target_idx(idx: u32, direction: u32, distance: u32) -> u32 {
    let x = i32(idx % GRID_DIM);
    let y = i32(idx / GRID_DIM);
    let dist = i32(distance);
    var dx = 0i;
    var dy = 0i;

    if (direction == JUMP_N) { dy = -dist; }
    else if (direction == JUMP_E) { dx = dist; }
    else if (direction == JUMP_S) { dy = dist; }
    else if (direction == JUMP_W) { dx = -dist; }

    let nx = (x + dx + i32(GRID_DIM)) % i32(GRID_DIM);
    let ny = (y + dy + i32(GRID_DIM)) % i32(GRID_DIM);
    return u32(ny) * GRID_DIM + u32(nx);
}

@compute @workgroup_size(16)
fn main(
    @builtin(workgroup_id) group_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let genome_idx = group_id.x;
    let cell_idx = local_id.x;
    let genome_offset = genome_idx * GENOME_SIZE;

    // Load genome into workgroup memory
    let g_cell = genomes[genome_offset + cell_idx];
    grid_a[cell_idx] = SimCell(g_cell.op, g_cell.dir_a, g_cell.dir_b, 0u, 0u);
    workgroupBarrier();

    // Accumulate correct bits across all test cases
    var correct_bits = 0u;

    // Run test cases: all 4 combinations of A, B
    for (var test = 0u; test < TEST_CASES; test++) {
        let a = (test >> 1u) & 1u;
        let b = test & 1u;
        let expected_sum = a ^ b;    // XOR
        let expected_carry = a & b;   // AND

        // Reset state
        grid_a[cell_idx].state = 0u;
        workgroupBarrier();

        // Run simulation
        for (var step = 0u; step < SIM_STEPS; step++) {
            let cell = grid_a[cell_idx];
            var res = 0u;

            if (cell.op < 16u) {
                let n_a = get_neighbor_idx(cell_idx, cell.dir_a);
                let n_b = get_neighbor_idx(cell_idx, cell.dir_b);

                let val_a = grid_a[n_a].state;
                let val_b = grid_a[n_b].state;

                let lut_idx = (val_a << 1u) | val_b;
                res = (cell.op >> lut_idx) & 1u;
            } else if (cell.op == OP_JUMP) {
                let jump_dest = get_jump_target_idx(cell_idx, cell.dir_a, cell.dir_b);
                res = grid_a[jump_dest].state;
            }

            // Fixed input cells receive the test bits
            if (cell_idx == INPUT_A) { res = a; }
            else if (cell_idx == INPUT_B) { res = b; }

            grid_b[cell_idx] = res;
            workgroupBarrier();

            grid_a[cell_idx].state = grid_b[cell_idx];
            workgroupBarrier();
        }

        // Check outputs
        if (cell_idx == 0u) {
            let actual_sum = grid_a[OUTPUT_SUM].state & 1u;
            let actual_carry = grid_a[OUTPUT_CARRY].state & 1u;

            if (actual_sum == expected_sum) {
                correct_bits += 1u;
            }
            if (actual_carry == expected_carry) {
                correct_bits += 1u;
            }
        }
        workgroupBarrier();
    }

    // Store result: total correct bits (max = 4 tests * 2 outputs = 8)
    if (cell_idx == 0u) {
        results[genome_idx] = correct_bits;
    }
}
