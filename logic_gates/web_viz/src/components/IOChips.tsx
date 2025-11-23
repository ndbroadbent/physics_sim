import React from 'react';
import '../circuit.css';

interface InputNumberBoxProps {
  label: string;
  value: number;
  onChange: (val: number) => void;
  bitCount?: number;
}

export const InputNumberBox: React.FC<InputNumberBoxProps> = ({ label, value, onChange, bitCount = 8 }) => {
  return (
    <div className="chip-box input-chip" style={{ zIndex: 10 }}> {/* Ensure z-index for interactivity */}
      <div className="chip-label">{label}</div>
      <input 
        type="number" 
        className="chip-input"
        min="0" 
        max="255" 
        value={value} 
        onChange={e => onChange(Math.min(255, Math.max(0, Number(e.target.value))))} 
      />
      {/* Output Ports on the Right */}
      <div className="chip-ports-right">
          {Array.from({ length: bitCount }).map((_, i) => {
              const bit = (value >> i) & 1;
              const statusClass = bit === 1 ? 'high' : 'low';
              return <div key={i} className={`chip-port ${statusClass}`} id={`${label}-out-${i}`} />
          })}
      </div>
    </div>
  );
};

interface OutputNumberBoxProps {
  label: string;
  value: number;
  overflow: boolean;
  bitCount?: number; // 8 data bits
}

export const OutputNumberBox: React.FC<OutputNumberBoxProps> = ({ label, value, overflow, bitCount = 8 }) => {
  return (
    <div className="chip-box output-chip">
      <div className="chip-label">{label}</div>
      <div className="chip-display">
          {value}
          {overflow && <div className="chip-overflow-warning">OVERFLOW</div>}
      </div>
      
      {/* Input Ports on the Left */}
      <div className="chip-ports-left">
          {Array.from({ length: bitCount + 1 }).map((_, i) => { // +1 for Overflow bit
              let bit = 0;
              if (i < bitCount) {
                  bit = (value >> i) & 1;
              } else {
                  bit = overflow ? 1 : 0;
              }
              const statusClass = bit === 1 ? 'high' : 'low';
              return <div key={i} className={`chip-port ${statusClass}`} id={`${label}-in-${i}`} />
          })}
      </div>
    </div>
  );
};
