import React, { useEffect, useMemo, useRef, useState } from 'react';
import { CircuitSimulator } from './logic/circuit';
import { buildAdder } from './logic/builder';
import { SimulatorProvider, useSimulator } from './hooks/useCircuit';
import { Gate } from './components/Gate';
import { FullAdder } from './components/FullAdder';
import { WireOverlay } from './components/WireOverlay';
import './circuit.css';
import './App.css';
import './extra.css';

function App() {
  // Initialize simulator once
  const [sim] = useState(() => new CircuitSimulator());
  const layout = useMemo(() => buildAdder(sim, 8), [sim]);
  
  const [valA, setValA] = useState(0);
  const [valB, setValB] = useState(0);
  const [delay, setDelay] = useState(50);

  const containerRef = useRef<HTMLDivElement>(null);

  // Sync delay
  useEffect(() => {
    sim.delayMs = delay;
  }, [delay, sim]);

  // Sync inputs
  const updateInputs = (a: number, b: number) => {
    // Ensure Cin is driven low (0)
    sim.setInput('Cin0', 0);

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
    sim.reset();
    setTimeout(() => {
        updateInputs(valA, valB);
    }, 100);
  };

  const toggleBit = (val: number, setVal: (v: number) => void, bit: number) => {
    setVal(val ^ (1 << bit));
  };

  return (
    <SimulatorProvider simulator={sim}>
      <div className="app-container">
        <div className="controls">
            <div className="control-group">
                <label>Input A</label>
                <input type="number" min="0" max="255" value={valA} onChange={e => setValA(Number(e.target.value))} />
            </div>
            <div className="control-group">
                 <label>Input B</label>
                 <input type="number" min="0" max="255" value={valB} onChange={e => setValB(Number(e.target.value))} />
            </div>
            <div className="control-group range">
                 <label>Speed ({delay}ms)</label>
                 <input type="range" min="10" max="500" value={delay} onChange={e => setDelay(Number(e.target.value))} />
            </div>
            <button className="btn-restart" onClick={handleReset}>RESTART</button>
        </div>

        <div className="circuit-container">
             <div className="inner-circuit" ref={containerRef}>
                 <WireOverlay containerRef={containerRef} />

                 <div className="section inputs-section" style={{ display: 'flex', justifyContent: 'center', gap: '40px' }}>
                     <div className="input-block">
                         <h3>A Bits (LSB -&gt; MSB)</h3>
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

                 <div className="section adders-section">
                     <h3>Full Adder Stack (8 Bits)</h3>
                     <div className="stack-container">
                         {layout.bits.map((group, i) => (
                             <FullAdder key={i} bit={i} groups={group} />
                         ))}
                     </div>
                 </div>

                 <div className="section outputs-section">
                     <h3>Output Sum Bits + Overflow</h3>
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

                 <ResultValue outputIds={layout.outputIds} coutId={layout.coutId} />
             </div>
        </div>
      </div>
    </SimulatorProvider>
  );
}

const ResultValue: React.FC<{ outputIds: string[], coutId: string }> = ({ outputIds, coutId }) => {
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

    return <div className="result-value">
        SUM: {val} {overflow && <span style={{color: '#f44', fontSize: '0.6em', marginLeft: '10px'}}>OVERFLOW</span>}
    </div>
}

export default App;