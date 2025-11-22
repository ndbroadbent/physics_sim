# Experiments, Questions, and Hypotheses

This file tracks "What If" scenarios and random ideas for the Physics Simulation engine.

## 1. Material Permutations
**Goal:** Find the theoretical maximum efficiency by varying the atomic lattice.
*   **Experiment:** Compare standard Silicon (Si) against Germanium (Ge), Gallium Arsenide (GaAs), and Cadmium Telluride (CdTe).
*   **Experiment:** "Periodic Table Scan" - Use the `Element` struct to try every possible combination of Group 13/15 and Group 12/16 dopants.
*   **Metric:** Power Conversion Efficiency (Current * Voltage / Input Energy).

## 2. The "Spectrum Shaping" Hypothesis
**Observation:**
*   In the current model, all photons are 2.5 eV.
*   Real sunlight is a broad spectrum (infrared to UV).
*   Photons with Energy < Band Gap pass through (wasted).
*   Photons with Energy >> Band Gap release their excess energy as heat (thermalization).

**Hypothesis:**
*   "Is there such a thing as too many photons?"
*   **Idea:** If we filter the sunlight to *only* allow a narrow band of photons (exactly matching the Band Gap + a tiny bit), we might reduce heat generation.
*   **Test:** Simulate a "Filter" layer that blocks specific energy levels. Does the remaining panel run cooler/more efficiently?
*   **Real-world parallel:** Multi-junction cells (splitting the spectrum) or optical filtering.

## 3. Temperature & Conductivity
**Question:** How does heat affect conductivity in this model?
*   **Physics Reality:**
    *   **Metals:** Heat = Lattice vibration (Phonons) = More scattering = *Lower* conductivity.
    *   **Semiconductors:** Heat = More thermal energy = More electrons jump the gap spontaneously = *Higher* conductivity (but more noise).
    *   **Solar Panels:** While conductivity might go up, the **Voltage ($V_{oc}$)** drops significantly as temperature rises. Net result: Efficiency usually drops.
*   **Simulation Plan:**
    *   Need to implement **Phonons** (lattice vibrations).
    *   Currently, `temperature` is implicit or 0 Kelvin.
    *   Need a `Temperature` variable that adds random noise to electron velocity.

## 4. "Too Many Photons" (Saturation)
**Hypothesis:**
*   Can we saturate the lattice?
*   If every valence electron is already excited to the conduction band, incoming photons have nothing to hit.
*   **Test:** Fire 1,000,000 photons at a small lattice. Does the absorption rate drop off?

## 5. Benchmark: The Gold Standard
**Goal:** Replicate the specs of a high-end commercial panel (e.g., SunPower Maxeon or similar).
*   **Target:** ~22-24% Efficiency.
*   **Configuration:** Monocrystalline Silicon, highly optimized doping, Back Surface Field (BSF), Anti-Reflective Coating.
*   **Challenge:** Can we tune our simulation parameters to match this 24% number exactly?
