import React from 'react';
import { useGateValue, useGateInputs } from '../hooks/useCircuit';
import '../circuit.css';

interface GateProps {
  id: string;
  label?: string;
  type?: 'NAND' | 'INPUT' | 'OUTPUT'; // Visual style
  onClick?: () => void;
}

export const Gate: React.FC<GateProps> = ({ id, label, type = 'NAND', onClick }) => {
  const value = useGateValue(id);
  const inputValues = useGateInputs(id);
  
  const valueClass = value === 1 ? 'high' : value === 0 ? 'low' : 'unknown';
  const typeClass = `gate-type-${type}`;
  const interactiveClass = onClick ? 'interactive' : '';

  // Helpers for input port classes
  const getPortClass = (val: number | null) => val === 1 ? 'high' : val === 0 ? 'low' : '';

  return (
    <div 
      id={`gate-${id}`} 
      className={`gate ${valueClass} ${typeClass} ${interactiveClass}`}
      onClick={onClick}
    >
      {type === 'INPUT' && (
          <div id={`gate-${id}-fake-in`} className="gate-port fake-in" />
      )}

      {type !== 'INPUT' && (
        <>
           {/* Input 0 */}
           <div 
             id={`gate-${id}-in-0`} 
             className={`gate-port in-0 ${getPortClass(inputValues[0])}`} 
           />
           {/* Input 1 (only for NAND) */}
           {type === 'NAND' && (
             <div 
               id={`gate-${id}-in-1`} 
               className={`gate-port in-1 ${getPortClass(inputValues[1])}`} 
             />
           )}
        </>
      )}
      
      <span className="gate-label">
        {type === 'NAND' ? 'NAND' : (label || id)}
      </span>

      {type !== 'OUTPUT' && (
        <div id={`gate-${id}-out`} className={`gate-port out ${valueClass}`} />
      )}
    </div>
  );
};
