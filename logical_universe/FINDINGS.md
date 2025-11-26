# Findings: Exploring the Edge of Chaos in the Logical Universe

This document summarizes observations and hypotheses from experiments conducted on the `logical_universe` simulation, focusing on the interplay between "Sustain" (XOR/XNOR) and "Carve" (NOR/NAND) logical operations.

## Simulation Setup

*   **Dimensions:** 128x128x128 3D grid.
*   **Initial State:** Two active "seed" bits in the 'Top' layer, 'Bottom' layer initialized to 'vacuum' (all 1s).
*   **Wiggle:** A chaotic pseudo-random spatial offset applied each frame to mix interactions.
*   **Vacuum State:** Top layer all 0s, Bottom layer all 1s. This state is stable under all logic operations.
*   **Survival Criterion:** The universe "dies" when it reaches the Vacuum State. "Survival Frames" indicates the number of frames until death, or the maximum frame limit if it persists.

## Key Observations & Discoveries

### 1. The 50/50 Baseline (Original Code)
*   **Ratio:** 50% Sustain : 50% Carve (Sustain operations every 2 frames).
*   **Observation:** The universe evolved from seeds into a widespread chaotic pattern (Class III) and then unexpectedly collapsed into the Vacuum State (Class I) around **Frame 4600**. This demonstrates a dissipative system with a fixed-point attractor.

### 2. Resonance is the Key Driver of Survival

Experiments sweeping the **Modulus** (frequency of Sustain operations) revealed that survival is not a simple function of the sustain percentage, but is highly sensitive to **Resonance** with the system's spatial or temporal properties.

*   **Setup:** Sustain Threshold = 1 (1 sustain per cycle). Modulus swept from 50 to 67.
*   **Results:**
    *   **Deadly Resonances:**
        *   **Modulus 64:** Died at **400** frames. (Note: 64 is exactly half the grid size 128).
        *   **Modulus 63:** Died at **300** frames.
        *   **Modulus 54:** Died at **600** frames.
        *   **Modulus 61 (Prime):** Died at **700** frames.
    *   **Stable Islands:**
        *   **Modulus 59 (Prime):** Survived **>20,000** frames.
    *   **The "Edge of Chaos" (Long Struggle):**
        *   **Modulus 58:** Died at **14,300** frames.
        *   **Modulus 60:** Died at **14,100** frames.
        *   **Modulus 67:** Died at **12,100** frames.

This proves that the system behaves like a **Resonant Dynamical System**. Specific frequencies of energy injection (Sustain) can actively disrupt stable structures (destructive interference) or allow them to persist (constructive interference or lack of disruption). "Primes are safer" was disproven by Modulus 61 (700) and Modulus 53 (2400).

### 3. Carve is Dissipative, Not Generative
*   We verified that `NOR(0,1)` and `NAND(1,0)` preserve the vacuum state. The system does not generate matter from nothing; it requires seeds.
*   Pure Carve (or low sustain) *should* lead to death, but specific resonance frequencies can accelerate this death drastically (from >20,000 frames down to 300 frames).

### 4. Fine Structure Constant Hypothesis
*   Initial hypothesis: $\alpha \approx 1/137 \approx 0.73\%$ might be a critical point.
*   Revised understanding: While the specific percentage matters, the **integer modulus** (frequency) is the dominant factor. The "magic" of 137 might be less about the ratio and more about the properties of the number 137 as a modulus in a discrete grid.

## Next Steps

*   **Visualize the "Edge":** Run a full visualization (with video/images) for one of the "Long Struggle" moduli (e.g., **Modulus 67** or **Modulus 58**). These universes fight for survival for over 10,000 frames before dying, which is the perfect candidate for observing Class IV-like structures (complex transients).
*   **Analyze Spatial Resonance:** Investigate why Modulus 64 (half grid size) is so lethal. Does it create a standing wave that cancels itself out?

## Visualizations

A visualization has been generated for **Modulus 58** (14,300 survival frames):
*   File: `logical_universe/logical_universe.mkv`
*   Parameters: Sustain Threshold 1 (Frequency ~1.7%), 15,000 frames.
*   Observation: This universe demonstrates the "Long Struggle" characteristic of the Edge of Chaos, persisting for a significant duration before succumbing to the resonance effects.

## Pulse vs. Burst Mode (Frequency Dependence)

Experiments comparing **Modulus 17** (Pulse Mode) and **Modulus 170** (Burst Mode) at equivalent sustain ratios revealed a critical distinction:

*   **Modulus 17 (Threshold 1 = 5.8%):** Sustain occurs 1 frame every 17. **Result: DEATH (Frame 3410).**
*   **Modulus 170 (Threshold 10 = 5.8%):** Sustain occurs 10 frames in a row every 170. **Result: LIFE (>10,000 frames).**

This implies that **High Frequency "Pulse" interruptions are lethal**, preventing structure formation or actively destroying it. **Low Frequency "Burst" injections are stabilizing**, likely because they inject a massive amount of material (XNOR is generative) which then has enough time (160 frames of Carve) to settle into stable oscillators before the next burst.

**The Paradox:**
*   **1% (Modulus 101) Pulse:** Dies.
*   **0.01% (Modulus 10000) Pulse:** Lives. (Extremely low frequency).

**Hypothesis:** There is a "Lethal Frequency Band" (around Modulus 17-100) where the sustain pulses interfere destructively with the natural decay/oscillation of the system. Below this frequency (Modulus 170+), the system stabilizes. Above this frequency (Modulus < 10? 50% sustain?), the system is chaotic noise.

**The "Edge of Chaos" is therefore a frequency boundary, roughly around Modulus 100-150.**