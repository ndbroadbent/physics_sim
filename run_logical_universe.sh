#!/bin/bash
set -e

# Ensure we are in the project directory
cd "$(dirname "$0")/logical_universe"

echo "Cleaning old frames..."
rm -rf frames/
mkdir -p frames

echo "Running Simulation..."
# Running in release mode for speed
cargo run --release
