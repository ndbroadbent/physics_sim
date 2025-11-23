import React from 'react';
import type { BitGroups } from '../logic/builder';
import { Gate } from './Gate';
import '../circuit.css';

export const FullAdder: React.FC<{ bit: number, groups: BitGroups }> = ({ bit, groups }) => {
  // Vertical Layout: HA1 (Top) -> HA2 (Middle) -> OR (Bottom)
  
  return (
    <div className="full-adder-container" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
      <div className="full-adder-box group-box" style={{ display: 'flex', flexDirection: 'column', gap: '20px', alignItems: 'center' }}>
        <div className="group-label">BIT {bit}</div>
        
            {/* HA1 Group */}
            <div className="ha-box group-box" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '10px' }}>
                <div className="group-label" style={{color: '#aaf'}}>HA1</div>
                
                {/* Level 1: AND Logic (n1 + c1) */}
                <div className="logic-group-box and-box" style={{ marginTop: '10px' }}>
                    <div className="logic-label">AND</div>
                    <div style={{ display: 'flex', gap: '10px' }}>
                        <Gate id={groups.ha1[0]} /> {/* n1 */}
                        
                        <div className="logic-group-box not-box">
                            <div className="logic-label">NOT</div>
                            <Gate id={groups.ha1[1]} /> {/* c1 */}
                        </div>
                    </div>
                </div>

                {/* XOR Group (n2, n3, s1) */}
                <div className="logic-group-box xor-box" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '10px' }}>
                    <div className="logic-label">XOR</div>
                    
                    {/* Level 2: XOR intermediates (n2, n3) */}
                    <div style={{ display: 'flex', gap: '20px' }}>
                         <Gate id={groups.ha1[2]} />
                         <Gate id={groups.ha1[3]} />
                    </div>

                    {/* Level 3: Sum (s1) */}
                    <Gate id={groups.ha1[4]} /> 
                </div>
            </div>

            {/* HA2 Group */}
            <div className="ha-box group-box" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '10px' }}>
                <div className="group-label" style={{color: '#faa'}}>HA2</div>

                 {/* Level 1: AND Logic */}
                <div className="logic-group-box and-box" style={{ marginTop: '10px' }}>
                    <div className="logic-label">AND</div>
                    <div style={{ display: 'flex', gap: '10px' }}>
                        <Gate id={groups.ha2[0]} />
                        
                        <div className="logic-group-box not-box">
                            <div className="logic-label">NOT</div>
                            <Gate id={groups.ha2[1]} />
                        </div>
                    </div>
                </div>

                {/* XOR Group */}
                <div className="logic-group-box xor-box" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '10px' }}>
                    <div className="logic-label">XOR</div>
                    
                    {/* Level 2: Intermediates */}
                    <div style={{ display: 'flex', gap: '20px' }}>
                         <Gate id={groups.ha2[2]} />
                         <Gate id={groups.ha2[3]} />
                    </div>

                    {/* Level 3: Sum */}
                    <Gate id={groups.ha2[4]} />
                </div>
            </div>

            {/* OR / Cout Group */}
            <div className="or-box group-box" style={{ width: '100%', display: 'flex', justifyContent: 'center' }}>
                <div className="group-label" style={{color: '#afa'}}>C-OUT</div>
                
                <div className="logic-group-box or-logic-box" style={{ marginTop: '10px' }}>
                    <div className="logic-label">OR</div>
                    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '10px' }}>
                        <div style={{ display: 'flex', gap: '10px' }}>
                            <div className="logic-group-box not-box">
                                <div className="logic-label">NOT</div>
                                <Gate id={groups.orGates[0]} />
                            </div>
                            <div className="logic-group-box not-box">
                                <div className="logic-label">NOT</div>
                                <Gate id={groups.orGates[1]} />
                            </div>
                        </div>
                        
                        <Gate id={groups.cout} label="NAND" />
                    </div>
                </div>
            </div>
      </div>
    </div>
  );
};