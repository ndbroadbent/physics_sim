
export type Bit = 0 | 1 | null;

export interface Gate {
  id: string;
  type: 'NAND' | 'INPUT' | 'OUTPUT';
  inputs: string[]; // Wire IDs
  outputs: string[]; // Wire IDs
  value: Bit;
  label?: string;
}

export interface Wire {
  id: string;
  sourceId: string; // Gate ID
  targetId: string; // Gate ID
  targetInputIndex: number; // 0 or 1 for NAND
  value: Bit;
}

export type CircuitState = {
  gates: Record<string, Gate>;
  wires: Record<string, Wire>;
};

export type CircuitListener = (event: CircuitEvent) => void;

export type CircuitEvent = 
  | { type: 'WIRE_UPDATE'; wireId: string; value: Bit }
  | { type: 'GATE_UPDATE'; gateId: string; value: Bit }
  | { type: 'RESET' };

export class CircuitSimulator {
  state: CircuitState;
  listeners: CircuitListener[] = [];
  queue: Array<{ type: 'process_gate'; gateId: string } | { type: 'propagate_wire'; wireId: string; value: Bit }> = [];
  delayMs: number = 500; // Default delay

  constructor() {
    this.state = { gates: {}, wires: {} };
  }

  addGate(id: string, type: 'NAND' | 'INPUT' | 'OUTPUT', label?: string) {
    this.state.gates[id] = { id, type, inputs: [], outputs: [], value: null, label };
  }

  addWire(sourceId: string, targetId: string, targetInputIndex: number) {
    const id = `${sourceId}->${targetId}:${targetInputIndex}`;
    this.state.wires[id] = { id, sourceId, targetId, targetInputIndex, value: null };
    
    // Link to gates
    if (this.state.gates[sourceId]) {
      this.state.gates[sourceId].outputs.push(id);
    }
    if (this.state.gates[targetId]) {
      this.state.gates[targetId].inputs[targetInputIndex] = id;
    }
    return id;
  }

  reset() {
    Object.values(this.state.gates).forEach(g => {
      g.value = null;
      this.emit({ type: 'GATE_UPDATE', gateId: g.id, value: null });
    });
    Object.values(this.state.wires).forEach(w => {
      w.value = null;
      this.emit({ type: 'WIRE_UPDATE', wireId: w.id, value: null });
    });
    this.queue = [];
  }

  setInput(gateId: string, value: Bit) {
    const gate = this.state.gates[gateId];
    if (!gate) return;
    
    if (gate.value !== value) {
      gate.value = value;
      this.emit({ type: 'GATE_UPDATE', gateId, value });
      // Propagate to outputs
      gate.outputs.forEach(wId => {
        this.scheduleWirePropagation(wId, value);
      });
    }
  }

  private scheduleWirePropagation(wireId: string, value: Bit) {
    // In a real event loop, we'd use setTimeout, but here we might want to control it manually
    // or just use setTimeout for the visual effect.
    // To keep it simple and synchronized with the UI, we will use setTimeout.
    
    // UI Effect first: "Wire starts filling" - handled by WIRE_UPDATE with null->value transition?
    // Actually, if we want to visualize travel, we need 2 states: "Start travel" and "Arrive".
    // For now, let's just say WIRE_UPDATE means "Source has pushed value into wire".
    // The UI will animate this. The actual *logical* arrival happens after delay.
    
    this.emit({ type: 'WIRE_UPDATE', wireId, value });
    
    setTimeout(() => {
      const wire = this.state.wires[wireId];
      if (!wire) return;
      
      // Arrival
      const targetGate = this.state.gates[wire.targetId];
      if (targetGate) {
        this.processGate(targetGate.id);
      }
    }, this.delayMs);
  }

  private processGate(gateId: string) {
    const gate = this.state.gates[gateId];
    if (!gate) return;
    if (gate.type === 'INPUT') return; // Inputs are set manually
    if (gate.type === 'OUTPUT') {
       // Just takes input 0
       const inputWireId = gate.inputs[0];
       const inputWire = this.state.wires[inputWireId];
       const newVal = inputWire ? inputWire.value : null;
       if (gate.value !== newVal) {
         gate.value = newVal;
         this.emit({ type: 'GATE_UPDATE', gateId, value: newVal });
       }
       return;
    }

    // NAND Logic
    // If any input is null, output is null (conceptually, or maybe 1? No, usually logic is undefined)
    // Let's say null propagates null.
    const val1 = this.getInputValue(gate, 0);
    const val2 = this.getInputValue(gate, 1);

    let newValue: Bit = null;
    if (val1 !== null && val2 !== null) {
      // NAND: 0,0->1; 0,1->1; 1,0->1; 1,1->0
      newValue = (val1 === 1 && val2 === 1) ? 0 : 1;
    } else {
      newValue = null;
    }

    if (gate.value !== newValue) {
      gate.value = newValue;
      this.emit({ type: 'GATE_UPDATE', gateId, value: newValue });
      // Propagate
      gate.outputs.forEach(wId => {
        this.scheduleWirePropagation(wId, newValue);
      });
    }
  }

  private getInputValue(gate: Gate, index: number): Bit {
    const wId = gate.inputs[index];
    if (!wId) return null;
    // We need the value that was *sent* to this wire.
    // In this simplified model, wire.value holds what source pushed.
    return this.state.wires[wId].value;
  }

  subscribe(listener: CircuitListener) {
    this.listeners.push(listener);
    return () => {
      this.listeners = this.listeners.filter(l => l !== listener);
    };
  }

  private emit(event: CircuitEvent) {
    this.listeners.forEach(l => l(event));
  }
}
