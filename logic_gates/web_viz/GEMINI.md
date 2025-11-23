# Logic Gates Web Viz

This project visualizes an 8-bit Ripple Carry Adder built entirely from NAND gates using React and Vite.

## Core Technologies
- **Runtime:** Bun (Use `bun` for all commands, do NOT use `npm`)
- **Framework:** React + Vite
- **Testing:** Vitest

## Commands
- `bun run dev`: Start development server
- `bun run build`: Build for production
- `bun run test`: Run tests ONCE (non-interactive)

## Project Structure
- `src/logic/circuit.ts`: Core event-driven circuit simulator.
- `src/logic/builder.ts`: Constructs the 8-bit adder topology.
- `src/components/`: React components for Gates, Wires, and layout.

## Important Notes
- **NAND Logic Only:** The simulation is strictly NAND-based. All other logic (NOT, AND, OR, XOR) is composed of NAND gates.
- **Initialization:** Ensure `Cin0` and all inputs are properly driven to `0` or `1` on reset to prevent "floating" (null) wires.
- **Testing:** The default `bun run test` command runs tests once.
