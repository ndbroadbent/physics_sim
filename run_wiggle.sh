#!/bin/bash
set -e

# Ensure we are in the project directory
cd "$(dirname "$0")/wiggle_life"

echo "Cleaning old frames..."
rm -rf frames/
mkdir -p frames

echo "Running Simulation..."
# Running in release mode for speed
cargo run --release

echo "Generating Video with ffmpeg..."
# -r 60: 60 frames per second
# -f image2: input format is image sequence
# -i frames/frame_%05d.png: input pattern
# -vcodec libx264: H.264 video codec
# -crf 25: Constant Rate Factor (quality, lower is better, 25 is decent for this)
# -pix_fmt yuv420p: standard pixel format for compatibility
# -y: overwrite output file
ffmpeg -r 30 -f image2 -s 1024x1024 -i frames/frame_%05d.png -vcodec libx264 -crf 25 -pix_fmt yuv420p -y wiggle_life.mp4

echo "Done! Video saved to wiggle_life/wiggle_life.mp4"
