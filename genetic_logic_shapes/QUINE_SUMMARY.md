# Quine Grid Analysis Summary

## Executive Summary
The "GRID QUINE SOLVED" result reported by the `quine_grid` binary is a **false positive**. The genetic algorithm did not evolve a self-replicating structure (Quine). Instead, it found a trivial solution that exploited three distinct bugs in the evaluation and fitness calculation logic.

The evolved structure is essentially a collection of **Void (Op 0)** cells connected to the global outputs, rather than a functional logic circuit that outputs its own structure.

## Root Causes

### 1. Trace vs. Evaluation Mismatch (The "Wrong Game" Bug)
The fitness function defined "Active Cells" using a trace seeded from the **Global Outputs** (indices 134, 11, etc.), but it evaluated correctness using the **Quine Outputs** (indices 150-153).
*   **Trace Logic:** Identified cells connected to Global Outputs.
*   **GPU Evaluation:** Calculated the input/output behavior of the 4x4 core (Quine logic).
*   **The Flaw:** The code verified the "Quine Behavior" of cells that were only active in the "Global Circuit". These two sets of cells are distinct.

### 2. Result Overflow (The "Zero Match" Bug)
The GPU returned 64 bits of result data (representing 16 test cases x 4 bits). The fitness function tried to check this result against all 64 cells of the genome.
*   **Logic:** `result_for_cell = (all_results >> (cell_index * 4)) & 0xF`
*   **Overflow:** For `cell_index >= 16`, the shift amount exceeds 64 bits. In Rust/CPU logic, this typically results in `0` (or undefined behavior that resolved to 0).
*   **Consequence:** Any cell with index 16 or greater was compared against `0`. If the cell's Op was `0` (Void), it registered as a perfect match.

### 3. GPU Simulation Masking
The WGSL shader only simulated the central 4x4 area (`x,y` in `6..10`).
*   **Consequence:** Any logic outside this 4x4 core was treated as Void by the GPU, producing `0` output.
*   But the **Global Trace** (on CPU) simulated the full 16x16 grid.
*   This allowed the GA to place "Active" structures outside the 4x4 core (which the CPU saw as active), while the GPU saw them as inactive (Output 0).

## The "Solution" Found
The Genetic Algorithm optimized for the following condition:
> "Find a structure that is connected to Global Outputs (Active), where every cell's Op Code matches the GPU Simulation Result."

Since the GPU Simulation Result for most cells (Index > 15 or outside 4x4) was effectively `0` due to the bugs above, the GA simply filled the grid with **Void Cells (Op 0)** that were connected to Global Outputs.
*   **Active?** Yes, connected to Global Output.
*   **Match?** Yes, `Target (0)` == `Result (0)`.

## Verification
An isolated CPU verification of the 4x4 "Quine Core" (the part that *should* have contained the logic) yielded **0 matches** out of 16 test cases. The core logic was non-functional, outputting a constant value (3) that matched none of the targets.

## Recommendations
To correctly evolve a Quine, the following fixes are required:
1.  **Align Trace and Evaluation:** The `trace_active` function must seed from the **Quine Outputs** (150-153), not Global Outputs.
2.  **Fix Result Extraction:** Ensure the GPU returns enough data for all 64 cells (needs 256 bits, not 64), or restrict the Quine goal to the 4x4 core.
3.  **Consistent Simulation:** Ensure GPU and CPU simulations cover the same grid area (8x8 vs 4x4).
