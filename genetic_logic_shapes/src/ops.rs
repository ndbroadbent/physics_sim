//! Operation definitions for logic grid cells
//!
//! Each cell has an `op` that determines its behavior:
//! - 0-15: 4-bit LUT logic gates (2 inputs → 1 output)
//! - 16: VOID (empty cell, no output)
//! - 17: JUMP wire (long-range connection)
//!
//! For JUMP wires:
//! - dir_a encodes direction: 0=N, 1=E, 2=S, 3=W (simplified cardinal)
//! - dir_b encodes distance: 2, 3, or 4 cells

// ============================================================================
// LOGIC GATES (0-15): 4-bit LUT encoding
// ============================================================================
// The op value IS the truth table: bit i = output when inputs = i
// Input encoding: (val_a << 1) | val_b gives index 0-3
//   val_a=0, val_b=0 → index 0 → bit 0
//   val_a=0, val_b=1 → index 1 → bit 1
//   val_a=1, val_b=0 → index 2 → bit 2
//   val_a=1, val_b=1 → index 3 → bit 3

pub const OP_FALSE: u32 = 0b0000;       // 0: Always 0
pub const OP_NOR: u32 = 0b0001;         // 1: A NOR B
pub const OP_AND_NOT_A: u32 = 0b0010;   // 2: (NOT A) AND B
pub const OP_NOT_A: u32 = 0b0011;       // 3: NOT A
pub const OP_AND_NOT_B: u32 = 0b0100;   // 4: A AND (NOT B)
pub const OP_NOT_B: u32 = 0b0101;       // 5: NOT B
pub const OP_XOR: u32 = 0b0110;         // 6: A XOR B
pub const OP_NAND: u32 = 0b0111;        // 7: A NAND B
pub const OP_AND: u32 = 0b1000;         // 8: A AND B
pub const OP_XNOR: u32 = 0b1001;        // 9: A XNOR B (equality)
pub const OP_B: u32 = 0b1010;           // 10: Pass B (wire)
pub const OP_OR_NOT_A: u32 = 0b1011;    // 11: (NOT A) OR B (A implies B)
pub const OP_A: u32 = 0b1100;           // 12: Pass A (wire)
pub const OP_OR_NOT_B: u32 = 0b1101;    // 13: A OR (NOT B) (B implies A)
pub const OP_OR: u32 = 0b1110;          // 14: A OR B
pub const OP_TRUE: u32 = 0b1111;        // 15: Always 1

// ============================================================================
// SPECIAL OPERATIONS (16+)
// ============================================================================

pub const OP_VOID: u32 = 16;            // Empty cell, outputs 0
pub const OP_JUMP: u32 = 17;            // Jump wire (long-range read)

// For OP_JUMP, we use dir_a and dir_b differently:
// dir_a: direction (0=N, 1=E, 2=S, 3=W) - using simplified 4-direction encoding
// dir_b: distance (2, 3, or 4)

// Direction constants for JUMP wires (simplified cardinal directions)
pub const JUMP_DIR_N: u32 = 0;
pub const JUMP_DIR_E: u32 = 1;
pub const JUMP_DIR_S: u32 = 2;
pub const JUMP_DIR_W: u32 = 3;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Check if an op is a logic gate (uses LUT)
pub fn is_logic_gate(op: u32) -> bool {
    op < 16
}

/// Check if an op is void
pub fn is_void(op: u32) -> bool {
    op == OP_VOID
}

/// Check if an op is a jump wire
pub fn is_jump(op: u32) -> bool {
    op == OP_JUMP
}

/// Check if an op is a simple wire (passes through one input)
pub fn is_wire(op: u32) -> bool {
    op == OP_A || op == OP_B
}

/// Get the name of an operation
pub fn op_name(op: u32) -> &'static str {
    match op {
        0 => "FALSE",
        1 => "NOR",
        2 => "AND_NOT_A",
        3 => "NOT_A",
        4 => "AND_NOT_B",
        5 => "NOT_B",
        6 => "XOR",
        7 => "NAND",
        8 => "AND",
        9 => "XNOR",
        10 => "B",
        11 => "OR_NOT_A",
        12 => "A",
        13 => "OR_NOT_B",
        14 => "OR",
        15 => "TRUE",
        16 => "VOID",
        17 => "JUMP",
        _ => "UNKNOWN",
    }
}

/// Get a symbol for an operation (for compact display)
pub fn op_symbol(op: u32) -> &'static str {
    match op {
        0 => "0",
        1 => "⊽",  // NOR
        2 => "↚",  // AND_NOT_A
        3 => "¬A",
        4 => "↛",  // AND_NOT_B
        5 => "¬B",
        6 => "⊕",  // XOR
        7 => "⊼",  // NAND
        8 => "∧",  // AND
        9 => "⊙",  // XNOR
        10 => "→B",
        11 => "→",  // implies
        12 => "→A",
        13 => "←",  // implied by
        14 => "∨",  // OR
        15 => "1",
        16 => "·",  // VOID
        17 => "⟿",  // JUMP
        _ => "?",
    }
}

/// Evaluate a logic gate given two input bits
pub fn eval_gate(op: u32, val_a: u32, val_b: u32) -> u32 {
    if op < 16 {
        let lut_idx = (val_a << 1) | val_b;
        (op >> lut_idx) & 1
    } else {
        0 // VOID and JUMP return 0 when evaluated as gates
    }
}

/// Calculate the target cell offset for a JUMP wire
/// Returns (dx, dy) offset from current cell
pub fn jump_offset(direction: u32, distance: u32) -> (i32, i32) {
    let dist = distance as i32;
    match direction {
        JUMP_DIR_N => (0, -dist),
        JUMP_DIR_E => (dist, 0),
        JUMP_DIR_S => (0, dist),
        JUMP_DIR_W => (-dist, 0),
        _ => (0, 0),
    }
}

/// Convert 8-direction encoding to (dx, dy)
pub fn dir_to_offset(dir: u32) -> (i32, i32) {
    match dir {
        0 => (0, -1),   // N
        1 => (1, -1),   // NE
        2 => (1, 0),    // E
        3 => (1, 1),    // SE
        4 => (0, 1),    // S
        5 => (-1, 1),   // SW
        6 => (-1, 0),   // W
        7 => (-1, -1),  // NW
        _ => (0, 0),
    }
}

/// Direction name for 8-direction encoding
pub fn dir_name(dir: u32) -> &'static str {
    match dir {
        0 => "N",
        1 => "NE",
        2 => "E",
        3 => "SE",
        4 => "S",
        5 => "SW",
        6 => "W",
        7 => "NW",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logic_gates() {
        // Test XOR
        assert_eq!(eval_gate(OP_XOR, 0, 0), 0);
        assert_eq!(eval_gate(OP_XOR, 0, 1), 1);
        assert_eq!(eval_gate(OP_XOR, 1, 0), 1);
        assert_eq!(eval_gate(OP_XOR, 1, 1), 0);

        // Test AND
        assert_eq!(eval_gate(OP_AND, 0, 0), 0);
        assert_eq!(eval_gate(OP_AND, 0, 1), 0);
        assert_eq!(eval_gate(OP_AND, 1, 0), 0);
        assert_eq!(eval_gate(OP_AND, 1, 1), 1);

        // Test OR
        assert_eq!(eval_gate(OP_OR, 0, 0), 0);
        assert_eq!(eval_gate(OP_OR, 0, 1), 1);
        assert_eq!(eval_gate(OP_OR, 1, 0), 1);
        assert_eq!(eval_gate(OP_OR, 1, 1), 1);

        // Test pass-through wires
        assert_eq!(eval_gate(OP_A, 0, 0), 0);
        assert_eq!(eval_gate(OP_A, 0, 1), 0);
        assert_eq!(eval_gate(OP_A, 1, 0), 1);
        assert_eq!(eval_gate(OP_A, 1, 1), 1);

        assert_eq!(eval_gate(OP_B, 0, 0), 0);
        assert_eq!(eval_gate(OP_B, 0, 1), 1);
        assert_eq!(eval_gate(OP_B, 1, 0), 0);
        assert_eq!(eval_gate(OP_B, 1, 1), 1);
    }

    #[test]
    fn test_jump_offsets() {
        assert_eq!(jump_offset(JUMP_DIR_N, 2), (0, -2));
        assert_eq!(jump_offset(JUMP_DIR_E, 3), (3, 0));
        assert_eq!(jump_offset(JUMP_DIR_S, 4), (0, 4));
        assert_eq!(jump_offset(JUMP_DIR_W, 2), (-2, 0));
    }
}
