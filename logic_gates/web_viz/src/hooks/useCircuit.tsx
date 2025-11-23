import React, { createContext, useContext, useSyncExternalStore } from 'react';
import { CircuitSimulator } from '../logic/circuit';

const SimulatorContext = createContext<CircuitSimulator | null>(null);

export const SimulatorProvider: React.FC<{ simulator: CircuitSimulator, children: React.ReactNode }> = ({ simulator, children }) => {
  return (
    <SimulatorContext.Provider value={simulator}>
      {children}
    </SimulatorContext.Provider>
  );
};

export const useSimulator = () => {
  const sim = useContext(SimulatorContext);
  if (!sim) throw new Error("useSimulator must be used within SimulatorProvider");
  return sim;
};

export const useGateValue = (gateId: string) => {
  const sim = useSimulator();
  
  return useSyncExternalStore(
    (callback) => {
      const unsubscribe = sim.subscribe((event) => {
        if (event.type === 'GATE_UPDATE' && event.gateId === gateId) {
          callback();
        }
        if (event.type === 'RESET') callback();
      });
      return unsubscribe;
    },
    () => sim.state.gates[gateId]?.value ?? null
  );
};

export const useWireValue = (wireId: string) => {
  const sim = useSimulator();
  
  return useSyncExternalStore(
    (callback) => {
      const unsubscribe = sim.subscribe((event) => {
        if (event.type === 'WIRE_UPDATE' && event.wireId === wireId) {
          callback();
        }
        if (event.type === 'RESET') callback();
      });
      return unsubscribe;
    },
    () => sim.state.wires[wireId]?.value ?? null
  );
};