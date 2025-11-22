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

## 6. The "Micro-Turbine" / Magnetic Dust Hypothesis
**Concept:**
*   Traditional power generation: Heat -> Steam -> Turbine -> Moving Magnet -> Current.
*   **Idea:** Shrink this to the micro/nano scale.
*   **Mechanism:** "Pushing" tiny magnets (e.g., magnetized iron dust or molecular magnets) through tiny semiconductor/metal channels using **Heat Energy** (Brownian motion or directed thermal gradient).
*   **Physics:** As these micro-magnets move past coils/wires (or through conductive fluids), they should induce a tiny EMF via Faraday's Law ($d\Phi_B/dt$).
*   **Question:** Can the chaotic thermal motion of magnetic particles be rectified into a directional current?
*   **Theoretical Link:** This sounds like a **Thermo-Magnetic Generator** or a variant of **Magnetohydrodynamics (MHD)** at the nano-scale.
*   **Challenge:** The Second Law of Thermodynamics (Maxwell's Demon). You can't extract work from random heat without a temperature difference (Hot side vs Cold side).
*   **Simulation Proposal:**
    *   Model a fluid of "Magnetic Particles".
    *   Apply a heat gradient (Hot Left -> Cold Right).
    *   Particles drift/diffuse from Hot to Cold.
    *   Place "Pick-up Coils" (simulated induction loops) along the path.
    *   Measure if a net current is induced.

## 7. Materials Science: Evolving Super-Materials
**Goal:** Use the genetic engine to design micro-structures (foams/lattices) with extreme properties.
*   **Target:** **Titanium/Aluminum Foam**.
*   **Metric:** Maximize Strength-to-Weight ratio.
*   **Physics:** Molecular Dynamics (MD) with interatomic potentials (Lennard-Jones / EAM).
*   **Simulation:** Grow a crystal lattice inside the evolved SDF -> Apply stress -> Measure strain/failure.

## 8. Battery Chemistry
**Goal:** Explore Aluminium-Ion (Al-Ion) batteries.
*   **Concept:** Al-Ion is theoretically safer and cheaper than Li-Ion.
*   **Simulation:** Model the diffusion of Al3+ ions through a porous cathode material.
*   **Challenge:** Finding a cathode structure that allows fast ion transport but remains stable. This is a perfect job for our **SDF Genetic Optimizer** (finding the best "sponge" for ions).

## 9. Room Temperature Superconductors
**Goal:** The holy grail of condensed matter physics.
*   **Concept:** Electron-Phonon coupling.
*   **Simulation:** This requires a quantum layer (Schrödinger equation or Ginzburg-Landau theory) on top of our particle system.
*   **Idea:** Can we find a lattice geometry that naturally "pairs" electrons (Cooper pairs) via phonon resonance?

## 10. "Light-as-Software" / Wave-Based Computing
**Goal:** A computer where both the **Data** and the **Program** are interacting beams of light.
*   **Concept:** The physical substrate is a "VM" (a resonant cavity or non-linear medium). The "Program" is a specific pattern of light that configures the medium dynamically via interference or non-linear effects. The "Data" is a second pattern that flows through this configuration.
*   **Mechanism:**
    *   **Program Beam:** Sets up standing waves or refractive index changes ($n(I)$) that define the logic gates.
    *   **Data Beam:** Flows through these "virtual wires".
    *   **Interaction:** Constructive/Destructive interference acts as the logic operation.
*   **Experiment:** **The Optical AND Gate**.
    *   Input A: Laser Pulse. Input B: Laser Pulse.
    *   Substrate: Non-linear Kerr medium.
    *   Logic: Can we arrange A and B such that Output exists IFF A and B collide?
*   **Simulation:** Requires a **2D FDTD (Finite Difference Time Domain)** wave solver. No particles. Just Maxwell's Equations.
*   **Design:** Manually design the wave collision geometry first (no genetic algorithms yet) to prove an AND gate is possible.

## 11. Resilience: ECC and Self-Calibration
**Goal:** Make the Optical Computer robust against real-world imperfections.
*   **Concept:** The physical substrate (glass/crystal) will never be perfect. It will have defects, and temperature will cause drift. We treat the hardware as a **Noisy Channel**.
*   **Strategy:**
    *   **Error Correcting Codes (ECC):** Encode 1 bit of logical data into N bits of physical signal (Redundancy/Orthogonality). Even if the optical gate fails 20% of the time, the logical bit survives.
    *   **Self-Calibration:** On startup, the "Program Beam" runs a test pattern. It measures the output distortion and "tunes" the input phase/direction to compensate for the specific defects of that specific chip.
*   **Simulation:**
    *   Take the **FDTD Optical Gate** (Exp #10).
    *   Introduce **Noise**: Randomly jitter the refractive index of the grid every frame.
    *   Implement an **ECC Layer** (Hamming code or Repetition) around the input/output.
    *   **Metric:** Bit Error Rate (BER) vs Noise Level. prove that ECC allows a "broken" machine to compute perfectly.
