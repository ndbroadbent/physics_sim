import struct
import numpy as np
import os

DIM_X = 128
DIM_Y = 128
DIM_Z = 128
VOL_SIZE = DIM_X * DIM_Y * DIM_Z

def read_dump(filename):
    with open(filename, 'rb') as f:
        data = f.read()
    
    # Each u32 is 4 bytes
    # Top layer first, then Bottom layer
    layer_bytes = VOL_SIZE * 4
    
    top_raw = data[:layer_bytes]
    bottom_raw = data[layer_bytes:]
    
    # Unpack
    # '<' for little-endian (standard on x86/arm usually, and wgpu usually matches host or is le)
    # actually strictly speaking we used to_le_bytes in rust
    top = np.frombuffer(top_raw, dtype=np.uint32)
    bottom = np.frombuffer(bottom_raw, dtype=np.uint32)
    
    return top.reshape((DIM_Z, DIM_Y, DIM_X)), bottom.reshape((DIM_Z, DIM_Y, DIM_X))

def analyze(frame):
    filename = f"dump_{frame}.bin"
    if not os.path.exists(filename):
        print(f"File {filename} not found.")
        return

    print(f"--- Analyzing Frame {frame} ---")
    top, bottom = read_dump(filename)
    
    # Top: 1 is active, 0 is vacuum
    # Bottom: 0 is active, 1 is vacuum (based on logic: 0,1 -> vacuum, 1,1 -> white, 0,0 -> purple)
    # Actually let's look at the renderer logic in main.rs:
    # (0, 1) => Transparent (Vacuum)
    # (0, 0) => Purple
    # (1, 1) => White
    # (1, 0) => Cyan
    
    # So "Active" non-vacuum stuff is when Top=1 OR Bottom=0.
    
    top_ones = np.sum(top)
    bottom_zeros = np.sum(1 - bottom) # bottom is 0 or 1
    
    print(f"Top Ones: {top_ones}")
    print(f"Bottom Zeros: {bottom_zeros}")
    
    # Find bounding box of activity
    active_mask = (top == 1) | (bottom == 0)
    
    if np.sum(active_mask) == 0:
        print("UNIVERSE IS EMPTY (Vacuum State)")
        return

    indices = np.argwhere(active_mask)
    z_min, y_min, x_min = indices.min(axis=0)
    z_max, y_max, x_max = indices.max(axis=0)
    
    print(f"Bounding Box: [{x_min}:{x_max}, {y_min}:{y_max}, {z_min}:{z_max}]")
    print(f"Volume: {(x_max-x_min)*(y_max-y_min)*(z_max-z_min)}")
    
    # ASCII Slice at Z center of the bounding box
    z_slice_idx = (z_min + z_max) // 2
    print(f"\nSlice at Z={z_slice_idx} (Crop {x_min}-{x_max}, {y_min}-{y_max}):")
    
    slice_top = top[z_slice_idx, y_min:y_max+1, x_min:x_max+1]
    slice_bot = bottom[z_slice_idx, y_min:y_max+1, x_min:x_max+1]
    
    for y in range(slice_top.shape[0]):
        line = ""
        for x in range(slice_top.shape[1]):
            t = slice_top[y, x]
            b = slice_bot[y, x]
            
            if t == 0 and b == 1: char = "." # Vacuum
            elif t == 0 and b == 0: char = "P" # Purple
            elif t == 1 and b == 1: char = "W" # White
            elif t == 1 and b == 0: char = "C" # Cyan
            else: char = "?"
            line += char
        print(line)
    
    # Check for repeating patterns or "fractal" nature?
    # Just simple check: are there 2x2 blocks of same color?
    
    print("\n")

frames = [4541, 4613, 4627, 4643]
for f in frames:
    analyze(f)
