import { CircuitSimulator } from './circuit';

export interface BitGroups {
  ha1: string[]; // IDs of gates in HA1
  ha2: string[];
  orGates: string[]; // The OR logic
  cout: string;
}

export interface AdderLayout {
  bits: BitGroups[];
  inputIdsA: string[];
  inputIdsB: string[];
  outputIds: string[];
  coutId: string;
}

export function buildAdder(sim: CircuitSimulator, numBits: number = 8): AdderLayout {
  // Inputs
  const inputIdsA: string[] = [];
  const inputIdsB: string[] = [];
  
  // Cin0 is a constant 0 input
  sim.addGate('Cin0', 'INPUT', 'Cin');
  sim.setInput('Cin0', 0);

  for (let i = 0; i < numBits; i++) {
    const idA = `A${i}`;
    const idB = `B${i}`;
    sim.addGate(idA, 'INPUT', `A${i}`);
    sim.addGate(idB, 'INPUT', `B${i}`);
    inputIdsA.push(idA);
    inputIdsB.push(idB);
  }

  const bits: BitGroups[] = [];
  let prevCout = 'Cin0';

  for (let bit = 0; bit < numBits; bit++) {
    const group: BitGroups = { ha1: [], ha2: [], orGates: [], cout: '' };
    const prefix = `b${bit}`;

    // Helpers to create gates and wires
    const createNand = (suffix: string, label: string = 'NAND') => {
      const id = `${prefix}_${suffix}`;
      sim.addGate(id, 'NAND', label);
      return id;
    };
    
    const connect = (src: string, dst: string, idx: number) => {
      sim.addWire(src, dst, idx);
    };

    // Inputs for this bit
    const A = `A${bit}`;
    const B = `B${bit}`;
    const Cin = prevCout;

    // HA1
    // ha1_n1 = NAND(A, B)
    const ha1_n1 = createNand('ha1_n1');
    group.ha1.push(ha1_n1);
    connect(A, ha1_n1, 0);
    connect(B, ha1_n1, 1);

    // c1 = NOT(ha1_n1) = NAND(ha1_n1, ha1_n1)  (This is the AND output of HA1)
    const c1 = createNand('c1', 'NOT'); 
    group.ha1.push(c1);
    connect(ha1_n1, c1, 0);
    connect(ha1_n1, c1, 1);

    // ha1_n2 = NAND(A, ha1_n1)
    const ha1_n2 = createNand('ha1_n2');
    group.ha1.push(ha1_n2);
    connect(A, ha1_n2, 0);
    connect(ha1_n1, ha1_n2, 1);

    // ha1_n3 = NAND(B, ha1_n1)
    const ha1_n3 = createNand('ha1_n3');
    group.ha1.push(ha1_n3);
    connect(B, ha1_n3, 0);
    connect(ha1_n1, ha1_n3, 1);

    // s1 = NAND(ha1_n2, ha1_n3) (Partial Sum)
    const s1 = createNand('s1');
    group.ha1.push(s1);
    connect(ha1_n2, s1, 0);
    connect(ha1_n3, s1, 1);


    // HA2
    // ha2_n1 = NAND(s1, Cin)
    const ha2_n1 = createNand('ha2_n1');
    group.ha2.push(ha2_n1);
    connect(s1, ha2_n1, 0);
    connect(Cin, ha2_n1, 1);

    // c2 = NOT(ha2_n1) = NAND(ha2_n1, ha2_n1)
    const c2 = createNand('c2', 'NOT');
    group.ha2.push(c2);
    connect(ha2_n1, c2, 0);
    connect(ha2_n1, c2, 1);

    // ha2_n2 = NAND(s1, ha2_n1)
    const ha2_n2 = createNand('ha2_n2');
    group.ha2.push(ha2_n2);
    connect(s1, ha2_n2, 0);
    connect(ha2_n1, ha2_n2, 1);

    // ha2_n3 = NAND(Cin, ha2_n1)
    const ha2_n3 = createNand('ha2_n3');
    group.ha2.push(ha2_n3);
    connect(Cin, ha2_n3, 0);
    connect(ha2_n1, ha2_n3, 1);

    // S_local = NAND(ha2_n2, ha2_n3) (Final Sum for this bit)
    const S_local = createNand('S');
    group.ha2.push(S_local);
    connect(ha2_n2, S_local, 0);
    connect(ha2_n3, S_local, 1);

    // OR Logic (Carry Out)
    // Cout = OR(c1, c2) = NAND(NOT(c1), NOT(c2))? 
    // Python script says: 
    // or_n1 inputs c1, c1 (so or_n1 = NOT(c1))
    // or_n2 inputs c2, c2 (so or_n2 = NOT(c2))
    // Cout inputs or_n1, or_n2 (so Cout = NAND(NOT c1, NOT c2) = OR(c1, c2))

    const or_n1 = createNand('or_n1', 'NOT');
    group.orGates.push(or_n1);
    connect(c1, or_n1, 0);
    connect(c1, or_n1, 1);

    const or_n2 = createNand('or_n2', 'NOT');
    group.orGates.push(or_n2);
    connect(c2, or_n2, 0);
    connect(c2, or_n2, 1);

    const Cout = createNand('Cout');
    group.cout = Cout; // Is strictly part of OR logic but we mark it specially
    group.orGates.push(Cout);
    connect(or_n1, Cout, 0);
    connect(or_n2, Cout, 1);

    prevCout = Cout;
    bits.push(group);
  }

  // Output Nodes (Visualization only, really)
  const outputIds: string[] = [];
  for (let i = 0; i < numBits; i++) {
    const sId = `S${i}`;
    sim.addGate(sId, 'OUTPUT', `S${i}`);
    // Connect from internal S gate
    sim.addWire(bits[i].ha2[4], sId, 0);
    outputIds.push(sId);
  }

  // Final Carry Out Output
  const coutId = 'Cout_Final';
  sim.addGate(coutId, 'OUTPUT', 'Overflow');
  sim.addWire(prevCout, coutId, 0);

  return { bits, inputIdsA, inputIdsB, outputIds, coutId };
}