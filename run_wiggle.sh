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

