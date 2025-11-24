import cv2
import numpy as np
import matplotlib.pyplot as plt
import zlib
import math
import sys
import csv

def calculate_entropy(counts, total_pixels):
    """Calculates Shannon entropy for the given counts."""
    entropy = 0
    for count in counts:
        if count > 0:
            p = count / total_pixels
            entropy -= p * math.log2(p)
    return entropy

def calculate_edge_density(frame_quantized):
    """Calculates the fraction of pixel boundaries that are different."""
    # Compare with right neighbor
    diff_x = frame_quantized[:, :-1] != frame_quantized[:, 1:]
    # Compare with bottom neighbor
    diff_y = frame_quantized[:-1, :] != frame_quantized[1:, :]

    total_edges = diff_x.size + diff_y.size
    matching_edges = np.sum(diff_x) + np.sum(diff_y)

    return matching_edges / total_edges

def analyze_video(video_path, sample_interval=100):
    cap = cv2.VideoCapture(video_path)

    if not cap.isOpened():
        print(f"Error: Could not open video {video_path}")
        return

    total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
    print(f"Analyzing {video_path} with {total_frames} frames. Sampling every {sample_interval} frames.")

    results = []
    frame_indices = []

    frame_count = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break

        if frame_count % sample_interval == 0:
            # --- 1. Quantization ---
            # Frame is BGR
            # Heuristic:
            # Sum of channels < 50 => Black (0)
            # Mean of channels > 200 => White (2)
            # Else => Blue (1)

            # Fast vectorization
            b, g, r = cv2.split(frame)
            sum_channels = b.astype(int) + g.astype(int) + r.astype(int)
            mean_channels = sum_channels / 3.0

            # Default to Blue (1)
            q_frame = np.ones_like(b, dtype=np.uint8)

            # Set Black (0)
            q_frame[sum_channels < 50] = 0

            # Set White (2)
            q_frame[mean_channels > 200] = 2

            # --- 2. Metrics ---
            unique, counts = np.unique(q_frame, return_counts=True)
            counts_dict = {0: 0, 1: 0, 2: 0}
            for val, count in zip(unique, counts):
                counts_dict[val] = count

            n_pixels = frame.shape[0] * frame.shape[1]

            p0 = counts_dict[0] / n_pixels
            p1 = counts_dict[1] / n_pixels
            p2 = counts_dict[2] / n_pixels

            # Energy: 0*p0 + 1*p1 + 2*p2
            energy = p1 + 2 * p2

            # Entropy
            entropy = calculate_entropy(counts_dict.values(), n_pixels)

            # Compression Ratio (Complexity)
            # We compress the quantized frame
            raw_bytes = q_frame.tobytes()
            compressed = zlib.compress(raw_bytes)
            comp_ratio = len(compressed) / len(raw_bytes)

            # Edge Density
            edge_dens = calculate_edge_density(q_frame)

            print(f"Frame {frame_count}: Energy={energy:.3f}, Entropy={entropy:.3f}, CompRatio={comp_ratio:.3f}")

            results.append({
                'frame': frame_count,
                'p0': p0, 'p1': p1, 'p2': p2,
                'energy': energy,
                'entropy': entropy,
                'comp_ratio': comp_ratio,
                'edge_density': edge_dens
            })
            frame_indices.append(frame_count)

        frame_count += 1

    cap.release()

    # --- Save CSV ---
    csv_file = 'analysis_stats.csv'
    with open(csv_file, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=results[0].keys())
        writer.writeheader()
        writer.writerows(results)
    print(f"Stats saved to {csv_file}")

    # --- Plotting ---
    fig, axs = plt.subplots(4, 1, figsize=(10, 12), sharex=True)

    # Energy
    axs[0].plot(frame_indices, [r['energy'] for r in results], color='orange')
    axs[0].set_ylabel('Total Energy (Normalized)')
    axs[0].set_title('System Energy over Time')

    # Entropy
    axs[1].plot(frame_indices, [r['entropy'] for r in results], color='purple')
    axs[1].set_ylabel('Shannon Entropy (bits)')
    axs[1].set_title('Information Entropy')

    # Compression Ratio
    axs[2].plot(frame_indices, [r['comp_ratio'] for r in results], color='green')
    axs[2].set_ylabel('Compression Ratio')
    axs[2].set_title('Complexity (zlib ratio)')

    # Edge Density
    axs[3].plot(frame_indices, [r['edge_density'] for r in results], color='red')
    axs[3].set_ylabel('Edge Density')
    axs[3].set_xlabel('Frame Number')
    axs[3].set_title('Spatial Structure (Edge Density)')

    plt.tight_layout()
    plt.savefig('analysis_plots.png')
    print("Plots saved to analysis_plots.png")

if __name__ == "__main__":
    video_file = "logical_universe_40k.mp4"
    if len(sys.argv) > 1:
        video_file = sys.argv[1]

    analyze_video(video_file)
