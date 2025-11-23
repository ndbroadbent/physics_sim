import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { CircuitSimulator, Bit } from './circuit';
import { buildAdder, AdderLayout } from './builder';

describe('CircuitSimulator and Adder Logic', () => {
  let sim: CircuitSimulator;
  let layout: AdderLayout;

  beforeEach(() => {
    sim = new CircuitSimulator();
    sim.delayMs = 0; 
    layout = buildAdder(sim, 8);
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  const setAndPropagate = (valA: number, valB: number) => {
    sim.reset();
    vi.runAllTimers(); // Clear reset events

    for (let i = 0; i < 8; i++) {
      sim.setInput(`A${i}`, ((valA >> i) & 1) as Bit);
      sim.setInput(`B${i}`, ((valB >> i) & 1) as Bit);
    }
    // Advance significantly
    vi.advanceTimersByTime(5000); 
  };

  const getOutputSum = (): number => {
    let sum = 0;
    layout.outputIds.forEach((id, i) => {
      if (sim.state.gates[id]?.value === 1) {
        sum += (1 << i);
      }
    });
    return sum;
  };

  const getCarryOut = (): Bit => {
    return sim.state.gates[layout.coutId]?.value ?? null;
  };

  it('should correctly add 1 + 0 = 1', () => {
    setAndPropagate(1, 0); 
    expect(getOutputSum()).toBe(1); 
    expect(getCarryOut()).toBe(0); 
  });

  it('should correctly add 2 + 2 = 4', () => {
    setAndPropagate(2, 2); 
    expect(getOutputSum()).toBe(4); 
    expect(getCarryOut()).toBe(0); 
  });

  it('should correctly add 255 + 1 = 0 with overflow', () => {
    setAndPropagate(255, 1); 
    expect(getOutputSum()).toBe(0); 
    expect(getCarryOut()).toBe(1); 
  });

  it('should evaluate ALL wires for 0 + 0', () => {
    setAndPropagate(0, 0);
    
    const unevaluatedWires = Object.values(sim.state.wires).filter(w => w.value === null);
    
    if (unevaluatedWires.length > 0) {
        console.error("Unevaluated Wires:", unevaluatedWires.map(w => w.id));
    }
    expect(unevaluatedWires.length).toBe(0);
    expect(getOutputSum()).toBe(0);
    expect(getCarryOut()).toBe(0);
  });
});