import React, { useEffect, useState } from 'react';
import { useSimulator } from '../hooks/useCircuit';

export const StaticWireOverlay: React.FC<{
  connections: Array<{ from: string, to: string, logicGateId?: string }>;
  containerRef: React.RefObject<HTMLDivElement | null>;
  zoom: number;
}> = ({ connections, containerRef, zoom }) => {
    const sim = useSimulator();
    const [paths, setPaths] = useState<React.ReactElement[]>([]);

    // Hack: useSyncExternalStore might not trigger if we just return sim.state.
    // We need to force update.
    // Actually, we can just use a simple effect that subscribes and forces update.
    const [version, setVersion] = useState(0);
    useEffect(() => {
        return sim.subscribe(() => setVersion(v => v + 1));
    }, [sim]);


    const update = () => {
        if (!containerRef.current || !zoom) return;
        const containerRect = containerRef.current.getBoundingClientRect();
        const newPaths: React.ReactElement[] = [];

        connections.forEach((conn, i) => {
            const fromEl = document.getElementById(conn.from);
            const toEl = document.getElementById(conn.to);

            if (fromEl && toEl) {
                const srcRect = fromEl.getBoundingClientRect();
                const dstRect = toEl.getBoundingClientRect();

                const p1 = {
                    x: (srcRect.left + srcRect.width / 2 - containerRect.left) / zoom,
                    y: (srcRect.top + srcRect.height / 2 - containerRect.top) / zoom
                };
                const p2 = {
                    x: (dstRect.left + dstRect.width / 2 - containerRect.left) / zoom,
                    y: (dstRect.top + dstRect.height / 2 - containerRect.top) / zoom
                };

                const cp1 = { x: p1.x + 30, y: p1.y };
                const cp2 = { x: p2.x - 30, y: p2.y };
                
                const d = `M ${p1.x} ${p1.y} C ${cp1.x} ${cp1.y}, ${cp2.x} ${cp2.y}, ${p2.x} ${p2.y}`;
                
                // Determine Color
                let stroke = "#444"; // Default off/unknown
                if (conn.logicGateId) {
                    const val = sim.state.gates[conn.logicGateId]?.value;
                    if (val === 1) stroke = "#4f4"; // High
                    else if (val === 0) stroke = "#933"; // Low
                }

                newPaths.push(
                    <path 
                        key={i} 
                        d={d} 
                        stroke={stroke} 
                        strokeWidth="1" // Thinner
                        fill="none" 
                        strokeLinecap="round"
                        style={{ transition: 'stroke 0.2s' }}
                    />
                );
            }
        });
        setPaths(newPaths);
    };

    useEffect(() => {
        const t = setTimeout(update, 200);
        window.addEventListener('resize', update);
        const observer = new ResizeObserver(update);
        if (containerRef.current) observer.observe(containerRef.current);
        return () => {
            clearTimeout(t);
            window.removeEventListener('resize', update);
            observer.disconnect();
        };
    }, [containerRef, connections, version, zoom]); // Re-run on version change

    return (
        <svg style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%', pointerEvents: 'none', zIndex: 0 }}>
            {paths}
        </svg>
    );
};