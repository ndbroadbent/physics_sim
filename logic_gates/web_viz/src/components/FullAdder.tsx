import React from 'react';
import type { BitGroups } from '../logic/builder';
import { Gate } from './Gate';
import '../circuit.css';

export const FullAdder: React.FC<{ bit: number, groups: BitGroups }> = ({ bit, groups }) => {
  // Layout: HA1, HA2, OR in a row?
  // Logic flow: A,B -> HA1 -> HA2 -> OR -> Cout
  // So Left-to-Right inside the box is good.
  
  return (
    <div className="full-adder-container" style={{ marginLeft: `${bit * 40}px` }}>
      <div className="full-adder-box group-box">
        <div className="group-label">BIT {bit}</div>
        
        <div style={{ display: 'flex', gap: '20px' }}>
            {/* HA1 Group */}
            <div className="ha-box group-box">
                <div className="group-label" style={{color: '#aaf'}}>HA1</div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                    {/* n1, c1 (NOT), n2, n3, s1 */}
                    <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
                        {/* AND Logic: NAND + NOT */}
                        <div className="logic-group-box and-box">
                            <div className="logic-label">AND</div>
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '15px' }}>
                                <Gate id={groups.ha1[0]} /> {/* n1 */}
                                
                                {/* c1 is a NOT gate */}
                                <div className="logic-group-box not-box">
                                    <div className="logic-label">NOT</div>
                                    <Gate id={groups.ha1[1]} />
                                </div>
                            </div>
                        </div>

                        <div style={{ display: 'flex', flexDirection: 'column', gap: '5px' }}>
                             <Gate id={groups.ha1[2]} />
                             <Gate id={groups.ha1[3]} />
                        </div>
                        <Gate id={groups.ha1[4]} /> {/* s1 */}
                    </div>
                </div>
            </div>

            {/* HA2 Group */}
            <div className="ha-box group-box">
                <div className="group-label" style={{color: '#faa'}}>HA2</div>
                 <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                     {/* n1, c2 (NOT), n2, n3, s_local */}
                    <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
                        {/* AND Logic: NAND + NOT */}
                        <div className="logic-group-box and-box">
                            <div className="logic-label">AND</div>
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '15px' }}>
                                <Gate id={groups.ha2[0]} />
                                
                                {/* c2 is a NOT gate */}
                                <div className="logic-group-box not-box">
                                    <div className="logic-label">NOT</div>
                                    <Gate id={groups.ha2[1]} />
                                </div>
                            </div>
                        </div>

                        <div style={{ display: 'flex', flexDirection: 'column', gap: '5px' }}>
                             <Gate id={groups.ha2[2]} />
                             <Gate id={groups.ha2[3]} />
                        </div>
                        <Gate id={groups.ha2[4]} />
                    </div>
                </div>
            </div>

            {/* OR / Cout Group */}
            <div className="or-box group-box">
                <div className="group-label" style={{color: '#afa'}}>C-OUT</div>
                
                {/* The 3 gates form an OR logic */}
                <div className="logic-group-box or-logic-box">
                    <div className="logic-label">OR</div>
                    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '10px' }}>
                        {/* or_n1 and or_n2 are NOT gates */}
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
    </div>
  );
};