#!/bin/bash
set -e

echo "=================================================="
echo "   Physics Simulation: Wave FDTD Engine"
echo "=================================================="

# 1. Clean up old frames (Critical to prevent old frames from merging)
echo "Cleaning up old frames..."
rm -rf physics_sim/frames
mkdir -p physics_sim/frames

# 2. Run the Rust Simulation
echo "Running Rust Simulation (physics_wave)..."
# We run inside the physics_sim directory so relative paths work
(cd physics_sim && cargo run --release --bin physics_wave)

echo "Simulation complete."

# 3. Generate Video (No motion blur, standard speed, high quality)
echo "Generating Video (wave_simulation.mp4)..."
ffmpeg -y -framerate 60 -i physics_sim/frames/frame_%04d.png -c:v libx264 -pix_fmt yuv420p physics_sim/wave_simulation.mp4

echo "=================================================="
echo "   Video Generated: physics_sim/wave_simulation.mp4"
echo "=================================================="