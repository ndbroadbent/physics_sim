import React, { useEffect, useState } from 'react';
import { useSimulator, useWireValue } from '../hooks/useCircuit';
import type { Wire } from '../logic/circuit';
import '../circuit.css';

interface Point { x: number; y: number }

// Extract Gate Type helper (in reality we should pass this in, but we can peek at sim)
// Or just heuristic: if wire is vertical?
// Better to check gate type.

const WirePath: React.FC<{ wire: Wire, p1: Point, p2: Point }> = ({ wire, p1, p2 }) => {
  const sim = useSimulator();
  const value = useWireValue(wire.id);
  const status = value === 1 ? 'high' : value === 0 ? 'low' : 'unknown';

  const sourceGate = sim.state.gates[wire.sourceId];
  const targetGate = sim.state.gates[wire.targetId];

  // Determine Control Points based on Gate Type (Port orientation)
  
  // Source:
  // INPUT -> Bottom Port -> Curve Down (+y)
  // NAND -> Right Port -> Curve Right (+x)
  let cp1 = { x: p1.x + 20, y: p1.y }; // Default Right
  if (sourceGate && sourceGate.type === 'INPUT') {
      cp1 = { x: p1.x, y: p1.y + 40 }; // Curve Down
  }

  // Target:
  // OUTPUT -> Top Port -> Curve Up (-y) from target
  // NAND -> Left Port -> Curve Left (-x) from target
  let cp2 = { x: p2.x - 20, y: p2.y }; // Default Left
  if (targetGate && targetGate.type === 'OUTPUT') {
      cp2 = { x: p2.x, y: p2.y - 40 }; // Curve Up
  }

  // Special case: Vertical long connection?
  // If source is INPUT (Bottom) and target is NAND (Left).
  // p1 is (x1, y1). p2 is (x2, y2). y2 > y1.
  // path: Down from x1,y1 -> Left into x2,y2.
  // CP1: x1, y1+40. CP2: x2-20, y2.
  // This works.
  
  const d = `M ${p1.x} ${p1.y} C ${cp1.x} ${cp1.y}, ${cp2.x} ${cp2.y}, ${p2.x} ${p2.y}`;

  return (
    <path d={d} className={`wire-path ${status}`} pathLength="1" />
  );
};

export const WireOverlay: React.FC<{ containerRef: React.RefObject<HTMLDivElement | null> }> = ({ containerRef }) => {
  const sim = useSimulator();
  const [positions, setPositions] = useState<Record<string, {start: Point, end: Point}>>({});

  // Use a resize observer to update positions
  useEffect(() => {
    const updatePositions = () => {
      if (!containerRef.current) return;
      const containerRect = containerRef.current.getBoundingClientRect();
      const newPositions: Record<string, {start: Point, end: Point}> = {};

      Object.values(sim.state.wires).forEach(wire => {
        // Check for both possible input port locations (NAND vs OUTPUT)
        // Actually just check DOM.
        const srcEl = document.getElementById(`gate-${wire.sourceId}-out`);
        const dstEl = document.getElementById(`gate-${wire.targetId}-in-${wire.targetInputIndex}`);

        if (srcEl && dstEl) {
          const srcRect = srcEl.getBoundingClientRect();
          const dstRect = dstEl.getBoundingClientRect();

          newPositions[wire.id] = {
            start: {
              x: srcRect.left + srcRect.width / 2 - containerRect.left,
              y: srcRect.top + srcRect.height / 2 - containerRect.top
            },
            end: {
              x: dstRect.left + dstRect.width / 2 - containerRect.left,
              y: dstRect.top + dstRect.height / 2 - containerRect.top
            }
          };
        }
      });
      setPositions(newPositions);
    };

    // Initial calculation after a short delay to ensure layout is done
    const t = setTimeout(updatePositions, 100);
    
    // Observer
    const observer = new ResizeObserver(updatePositions);
    if (containerRef.current) {
        observer.observe(containerRef.current);
    }
    
    window.addEventListener('resize', updatePositions);

    return () => {
      clearTimeout(t);
      observer.disconnect();
      window.removeEventListener('resize', updatePositions);
    };
  }, [sim, containerRef]); 

  return (
    <svg className="wire-overlay" style={{
      position: 'absolute', top: 0, left: 0, width: '100%', height: '100%', pointerEvents: 'none', zIndex: 1
    }}>
      {Object.values(sim.state.wires).map(wire => {
        const pos = positions[wire.id];
        if (!pos) return null;
        return <WirePath key={wire.id} wire={wire} p1={pos.start} p2={pos.end} />;
      })}
    </svg>
  );
};
