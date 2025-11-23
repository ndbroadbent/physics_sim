import React, { useEffect, useState } from 'react';

// Draws static "UI" wires between two sets of elements identified by ID patterns.
// e.g. "INPUT A-out-0" -> "A0-in-0" (Wait, A0 input gate has input port?) 
// No, Input Gates are sources. They have OUTPUT ports.
// The Chip is the *User Interface* source. The Gate is the *Circuit* source.
// Conceptually: Chip -> Gate.
// Gate type=INPUT has a port class "out" at bottom (modified in css).
// We need a place to connect TO on the Input Gate. 
// Let's assume we connect Chip-Right to Gate-Left (or Top?).
// Input Gates don't have Input Ports in the sim. Visual only.
// We can target the center of the gate or add a "fake" input port to the Gate component visual.

export const StaticWireOverlay: React.FC<{
  connections: Array<{ from: string, to: string, color?: string }>;
  containerRef: React.RefObject<HTMLDivElement | null>;
}> = ({ connections, containerRef }) => {
    const [paths, setPaths] = useState<React.ReactElement[]>([]);

    const update = () => {
        if (!containerRef.current) return;
        const containerRect = containerRef.current.getBoundingClientRect();
        const newPaths: React.ReactElement[] = [];

        connections.forEach((conn, i) => {
            const fromEl = document.getElementById(conn.from);
            // For 'to', if it's a Gate ID, we might want to target the element itself or a specific sub-element.
            // If 'to' is "gate-A0", we target center.
            const toEl = document.getElementById(conn.to);

            if (fromEl && toEl) {
                const srcRect = fromEl.getBoundingClientRect();
                const dstRect = toEl.getBoundingClientRect();

                const p1 = {
                    x: srcRect.left + srcRect.width / 2 - containerRect.left,
                    y: srcRect.top + srcRect.height / 2 - containerRect.top
                };
                const p2 = {
                    x: dstRect.left + dstRect.width / 2 - containerRect.left,
                    y: dstRect.top + dstRect.height / 2 - containerRect.top
                };

                // Simple Bezier
                // Right to Left?
                // Chip is left, Gate is right.
                // p1 is Left (Chip Right Port). p2 is Right (Gate).
                const cp1 = { x: p1.x + 30, y: p1.y };
                const cp2 = { x: p2.x - 30, y: p2.y };
                
                const d = `M ${p1.x} ${p1.y} C ${cp1.x} ${cp1.y}, ${cp2.x} ${cp2.y}, ${p2.x} ${p2.y}`;
                
                newPaths.push(
                    <path 
                        key={i} 
                        d={d} 
                        stroke={conn.color || "#444"} 
                        strokeWidth="2" 
                        fill="none" 
                        strokeLinecap="round"
                    />
                );
            }
        });
        setPaths(newPaths);
    };

    useEffect(() => {
        // Update on mount/resize
        const t = setTimeout(update, 200);
        window.addEventListener('resize', update);
        const observer = new ResizeObserver(update);
        if (containerRef.current) observer.observe(containerRef.current);
        return () => {
            clearTimeout(t);
            window.removeEventListener('resize', update);
            observer.disconnect();
        };
    }, [containerRef, connections]); // Re-run if connections change

    return (
        <svg style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%', pointerEvents: 'none', zIndex: 0 }}>
            {paths}
        </svg>
    );
};