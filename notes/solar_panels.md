how do solar panels actually work at the quantum physics level, and what material are they made of? how is the direction of current determined?

----

A solar cell is basically a quantum-engineered diode that turns incoming photons into separated electron-hole pairs and then forces those charges to move in one preferred direction. Three pieces: the material, the quantum effect, and the built-in direction of current.

## Material

Most commercial panels use crystalline silicon (mono or poly). Others use thin-film semiconductors like CdTe or CIGS, and some high-end concentrator cells use multi-junction III-V semiconductors (GaAs, InGaP, etc.). The key property is a tunable **band gap**.

A semiconductor has:

* A **valence band** (electrons bound in atoms)
* A **conduction band** (electrons free to move)
* A **band gap** (energy difference between them)

Photons with energy >= the band gap can promote an electron up into the conduction band.

## Quantum part: photon absorption and carrier creation

At the quantum level, sunlight is a stream of photons each carrying energy E=hf. When one hits the semiconductor:

1. If the photon energy is below the band gap, it just passes through.
2. If it matches or exceeds the band gap, it’s absorbed.
3. Absorption excites an electron into the conduction band.
4. The “hole” it leaves behind remains in the valence band.

So a single absorbed photon produces an **electron-hole pair**. This is a quantum transition between discrete energy bands.

These carriers don’t naturally flow in any direction. They move randomly unless an electric field separates them.

## Why current has a single direction

This is the engineered trick: the cell is built as a **p-n junction**, which creates an internal electric field.

* The top layer is doped **n-type** (extra electrons).
* The bottom layer is doped **p-type** (extra holes).

Where they meet, electrons diffuse into the p-side and holes diffuse into the n-side until charges build up. That charge buildup creates a **built-in electric field** pointing from the n-side toward the p-side.

This field is fixed inside the device. It means:

* Electrons are pushed one way.
* Holes are pushed the opposite way.

So when a photon creates an electron-hole pair near this junction, the field **rips them apart** before they can recombine:

* Electrons are swept into the n-layer.
* Holes are swept into the p-layer.

With external wires attached, electrons flow out of the n-side, through your load, and back into the p-side. That is the current direction.

## Putting it together

1. Photon hits semiconductor.
2. Quantum absorption event lifts an electron across the band gap.
3. Internal electric field of the p-n junction forces charges to separate.
4. The external circuit provides the path, so electrons flow in one fixed direction.

If you want more detail, I can zoom into the band-structure math, the depletion region field, or how recombination and efficiency limits (Shockley-Queisser) come into play.
