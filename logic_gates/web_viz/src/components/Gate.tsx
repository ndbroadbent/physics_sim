import React from 'react';
import { useGateValue } from '../hooks/useCircuit';
import '../circuit.css';

interface GateProps {
  id: string;
  label?: string;
  type?: 'NAND' | 'INPUT' | 'OUTPUT'; // Visual style
  onClick?: () => void;
}

export const Gate: React.FC<GateProps> = ({ id, label, type = 'NAND', onClick }) => {
  const value = useGateValue(id);
  
  const valueClass = value === 1 ? 'high' : value === 0 ? 'low' : 'unknown';
  const typeClass = `gate-type-${type}`;
  const interactiveClass = onClick ? 'interactive' : '';

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
           {/* Assume max 2 inputs for NAND. INPUT has 0. OUTPUT has 1. */}
           <div id={`gate-${id}-in-0`} className="gate-port in-0" />
           {type === 'NAND' && <div id={`gate-${id}-in-1`} className="gate-port in-1" />}
        </>
      )}
      
      <span className="gate-label">
        {type === 'NAND' ? 'NAND' : (label || id)}
      </span>

      {type !== 'OUTPUT' && (
        <div id={`gate-${id}-out`} className="gate-port out" />
      )}
    </div>
  );
};
