import React, { useEffect, useMemo, useRef, useState } from 'react';
import { CircuitSimulator } from './logic/circuit';
import { buildAdder } from './logic/builder';
import { SimulatorProvider, useSimulator } from './hooks/useCircuit';
import { Gate } from './components/Gate';
import { FullAdder } from './components/FullAdder';
import { WireOverlay } from './components/WireOverlay';
import { StaticWireOverlay } from './components/StaticWireOverlay';
import { InputNumberBox, OutputNumberBox } from './components/IOChips';
import './circuit.css';
import './App.css';
import './extra.css';
import './chips.css';

function App() {
  // Initialize simulator once
  const [sim] = useState(() => new CircuitSimulator());
  const layout = useMemo(() => buildAdder(sim, 8), [sim]);
  
  const [valA, setValA] = useState(0);
  const [valB, setValB] = useState(0);
  const [delay, setDelay] = useState(50);

  // Calculate static connections for UI wires
  // We pass 'logicGateId' to tell the overlay which gate's value determines the color.
  const staticConnections = useMemo(() => {
      const conns = [];
      // Connect Input A Chip to A Gates
      for (let i = 0; i < 8; i++) {
          conns.push({
              from: `INPUT A-out-${i}`,
              to: `gate-A${i}-fake-in`,
              logicGateId: `A${i}` // Color determined by Gate A{i}
          });
      }
      // Connect Input B Chip to B Gates
      for (let i = 0; i < 8; i++) {
          conns.push({
              from: `INPUT B-out-${i}`,
              to: `gate-B${i}-fake-in`,
              logicGateId: `B${i}`
          });
      }
      // Connect Output Gates to Sum Chip
      for (let i = 0; i < 8; i++) {
          conns.push({
              from: `gate-S${i}`,
              to: `SUM-in-${i}`,
              logicGateId: `S${i}`
          });
      }
      // Overflow
      conns.push({
          from: `gate-${layout.coutId}`,
          to: `SUM-in-8`,
          logicGateId: layout.coutId
      });

      return conns;
  }, [layout]);

  const containerRef = useRef<HTMLDivElement>(null);

  // Sync delay
  useEffect(() => {
    sim.delayMs = delay;
  }, [delay, sim]);

  // Initial setup
  useEffect(() => {
      sim.reset();
  }, [sim]);
  
  // ... rest of file ...

  // Sync inputs
  const updateInputs = (a: number, b: number) => {
    for (let i = 0; i < 8; i++) {
      const bitA = (a >> i) & 1;
      const bitB = (b >> i) & 1;
      // @ts-ignore
      sim.setInput(`A${i}`, bitA);
      // @ts-ignore
      sim.setInput(`B${i}`, bitB);
    }
  };

  useEffect(() => {
    updateInputs(valA, valB);
  }, [valA, valB, sim]);

  const handleReset = () => {
    // Disable transitions for instant reset
    document.body.classList.add('no-transitions');
    
    sim.reset();
    
    // Re-enable transitions after a brief moment to allow DOM to update
    requestAnimationFrame(() => {
        setTimeout(() => {
            document.body.classList.remove('no-transitions');
            // Restart flow
            setTimeout(() => {
                updateInputs(valA, valB);
            }, 50);
        }, 50);
    });
  };

  const handleClear = () => {
      setValA(0);
      setValB(0);
  };

  const toggleBit = (val: number, setVal: (v: number) => void, bit: number) => {
    setVal(val ^ (1 << bit));
  };

  // Speed Slider Logic: Invert
  // Slider Left (10) -> Slow (500ms)
  // Slider Right (500) -> Fast (10ms)
  const sliderVal = 510 - delay;
  const handleSpeedChange = (v: number) => {
      setDelay(510 - v);
  };

  return (
    <SimulatorProvider simulator={sim}>
      <div className="app-container">
        {/* Moved controls to simple floating bar or integrated? User wanted boxes in graph. 
            We'll keep speed/reset floating for utility. */}
        <div className="controls" style={{ position: 'absolute', top: 0, right: 0, width: 'auto', background: 'transparent', border: 'none' }}>
            <div className="control-group range" style={{ background: '#111', padding: '10px', borderRadius: '0 0 0 8px', border: '1px solid #333' }}>
                 <label>Speed</label>
                 <input 
                    type="range" 
                    min="10" 
                    max="500" 
                    value={sliderVal} 
                    onChange={e => handleSpeedChange(Number(e.target.value))} 
                 />
                 <button className="btn-restart" onClick={handleClear} style={{ marginLeft: '10px', background: '#666' }}>CLEAR</button>
                 <button className="btn-restart" onClick={handleReset} style={{ marginLeft: '10px' }}>RESET</button>
            </div>
        </div>

        <div className="circuit-container">
             <div className="inner-circuit" ref={containerRef}>
                 <WireOverlay containerRef={containerRef} />
                 <StaticWireOverlay connections={staticConnections} containerRef={containerRef} />

                 <div className="section inputs-section" style={{ display: 'flex', justifyContent: 'center', gap: '60px', alignItems: 'flex-start' }}>
                     {/* Input A Block */}
                     <div style={{ display: 'flex', gap: '20px', alignItems: 'center' }}>
                        <InputNumberBox label="INPUT A" value={valA} onChange={setValA} />
                        
                        <div className="input-block">
                            <h3>A Bits</h3>
                            <div className="bits-row">
                                {layout.inputIdsA.map((id, i) => (
                                    <Gate 
                                        key={id} 
                                        id={id} 
                                        type="INPUT" 
                                        onClick={() => toggleBit(valA, setValA, i)}
                                    />
                                ))}
                            </div>
                        </div>
                     </div>
                     
                     {/* Input B Block */}
                     <div style={{ display: 'flex', gap: '20px', alignItems: 'center' }}>
                        <InputNumberBox label="INPUT B" value={valB} onChange={setValB} />

                        <div className="input-block">
                            <h3>B Bits</h3>
                            <div className="bits-row">
                                {layout.inputIdsB.map((id, i) => (
                                    <Gate 
                                        key={id} 
                                        id={id} 
                                        type="INPUT" 
                                        onClick={() => toggleBit(valB, setValB, i)}
                                    />
                                ))}
                            </div>
                        </div>
                     </div>
                 </div>

                 <div className="section adders-section">
                     <h3>Full Adder Stack (8 Bits)</h3>
                     <div className="stack-container">
                         {layout.bits.map((group, i) => (
                             <FullAdder key={i} bit={i} groups={group} />
                         ))}
                     </div>
                 </div>

                 <div className="section outputs-section" style={{ display: 'flex', justifyContent: 'center', gap: '40px', alignItems: 'center' }}>
                     <div style={{ border: '1px dashed #555', padding: '20px', borderRadius: '8px', position: 'relative' }}>
                         <div className="group-label">OUTPUT BITS</div>
                         <div className="bits-row" style={{ justifyContent: 'center' }}>
                             {layout.outputIds.map((id, i) => (
                                 <div key={id} className="output-bit-wrapper">
                                     <Gate id={id} type="OUTPUT" />
                                     <div className="bit-label">2^{i}</div>
                                 </div>
                             ))}
                             {/* Divider */}
                             <div style={{ width: '20px', borderLeft: '1px dashed #555', margin: '0 10px' }}></div>
                             
                             <div className="output-bit-wrapper">
                                 <Gate id={layout.coutId} type="OUTPUT" label="OVF" />
                                 <div className="bit-label">CARRY</div>
                             </div>
                         </div>
                     </div>

                     {/* Sum Box */}
                     <ResultBox outputIds={layout.outputIds} coutId={layout.coutId} />
                 </div>
             </div>
        </div>
      </div>
    </SimulatorProvider>
  );
}

const ResultBox: React.FC<{ outputIds: string[], coutId: string }> = ({ outputIds, coutId }) => {
    const sim = useSimulator();
    const [val, setVal] = useState(0);
    const [overflow, setOverflow] = useState(false);
    
    useEffect(() => {
        const update = () => {
            let sum = 0;
            outputIds.forEach((id, i) => {
                const v = sim.state.gates[id]?.value;
                if (v === 1) sum += (1 << i);
            });
            setVal(sum);

            const ovf = sim.state.gates[coutId]?.value;
            setOverflow(ovf === 1);
        };
        
        const unsub = sim.subscribe(() => update());
        update();
        return unsub;
    }, [sim, outputIds, coutId]);

    return <OutputNumberBox label="SUM" value={val} overflow={overflow} />;
}

export default App;