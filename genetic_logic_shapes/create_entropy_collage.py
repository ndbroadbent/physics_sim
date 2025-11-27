#!/usr/bin/env python3
"""
Create a collage of images with the highest Shannon entropy.
Analyzes all PNG images in evolution_output/, selects top 64 by entropy,
shuffles them, and creates an 8x8 grid collage.
"""

import glob
import random
from pathlib import Path

import numpy as np
from PIL import Image
from scipy.stats import entropy


def calculate_shannon_entropy(image_path):
    """
    Calculate Shannon entropy of an image.
    Higher entropy indicates more visual complexity/randomness.
    """
    img = Image.open(image_path).convert('RGB')
    img_array = np.array(img)

    # Flatten to 1D and calculate histogram
    pixel_values = img_array.flatten()
    hist, _ = np.histogram(pixel_values, bins=256, range=(0, 256))

    # Normalize to get probability distribution
    hist = hist / hist.sum()

    # Calculate Shannon entropy
    return entropy(hist, base=2)


def apply_pastel_filter(img):
    """
    Convert harsh RGB values (black/full red/green/blue) to pastel colors.
    Maps each RGB channel to a pastel range.
    """
    img_array = np.array(img, dtype=np.float32)

    # Define color palette similar to GitHub syntax highlighting
    # Black (0,0,0) -> deep purple/navy
    # Red (255,0,0) -> warm red/coral
    # Green (0,255,0) -> rich green
    # Blue (0,0,255) -> vibrant blue

    pastel_palette = {
        (0, 0, 0): (22, 27, 34),         # Black -> GitHub dark background
        (255, 0, 0): (248, 81, 73),      # Red -> GitHub red
        (0, 255, 0): (87, 171, 90),      # Green -> GitHub green
        (0, 0, 255): (88, 166, 255),     # Blue -> GitHub blue
    }

    # Create output array
    result = np.zeros_like(img_array)

    # Process each pixel
    for y in range(img_array.shape[0]):
        for x in range(img_array.shape[1]):
            r, g, b = img_array[y, x]

            # Determine which primary color this is closest to
            if r == 0 and g == 0 and b == 0:
                # Black
                result[y, x] = pastel_palette[(0, 0, 0)]
            elif r > 0 and g == 0 and b == 0:
                # Red
                result[y, x] = pastel_palette[(255, 0, 0)]
            elif r == 0 and g > 0 and b == 0:
                # Green
                result[y, x] = pastel_palette[(0, 255, 0)]
            elif r == 0 and g == 0 and b > 0:
                # Blue
                result[y, x] = pastel_palette[(0, 0, 255)]
            else:
                # Fallback for any mixed colors (shouldn't happen based on description)
                result[y, x] = [r, g, b]

    return Image.fromarray(result.astype(np.uint8))


def main():
    # Find all PNG files (excluding generated collages)
    output_dir = Path('/Users/ndbroadbent/code/physics_simulations/genetic_logic_shapes/evolution_output')
    all_images = list(output_dir.glob('*.png'))
    image_paths = [p for p in all_images if not p.name.startswith('entropy_')]

    print(f"Found {len(image_paths)} images (excluding {len(all_images) - len(image_paths)} generated collages)")

    # Calculate entropy for each image
    print("Calculating Shannon entropy for each image...")
    image_entropy = []
    for i, img_path in enumerate(image_paths):
        if i % 20 == 0:
            print(f"  Processing {i}/{len(image_paths)}...")
        ent = calculate_shannon_entropy(img_path)
        image_entropy.append((img_path, ent))

    # Sort by entropy (descending) and select top 100
    image_entropy.sort(key=lambda x: x[1], reverse=True)
    top_100 = image_entropy[:100]

    print(f"\nTop 100 images by entropy:")
    for img_path, ent in top_100[:10]:
        print(f"  {img_path.name}: {ent:.4f}")
    print(f"  ...")
    print(f"  {top_100[-1][0].name}: {top_100[-1][1]:.4f}")

    # Shuffle the selected images
    selected_paths = [img_path for img_path, _ in top_100]
    random.shuffle(selected_paths)

    # Load all images, apply pastel filter, and determine grid cell size
    print("\nLoading images, applying pastel filter, and creating collage...")
    images = [apply_pastel_filter(Image.open(path)) for path in selected_paths]

    # Assume all images are the same size (use first image as reference)
    cell_width, cell_height = images[0].size

    # Create 10x10 grid
    grid_size = 10
    collage_width = cell_width * grid_size
    collage_height = cell_height * grid_size

    collage = Image.new('RGB', (collage_width, collage_height))

    # Paste images into grid
    for idx, img in enumerate(images):
        row = idx // grid_size
        col = idx % grid_size
        x = col * cell_width
        y = row * cell_height
        collage.paste(img, (x, y))

    # Save collage
    output_path = output_dir / 'entropy_collage_10x10_pastel.png'
    collage.save(output_path, quality=95)

    print(f"\nCollage saved to: {output_path}")
    print(f"Dimensions: {collage_width}x{collage_height}")


if __name__ == '__main__':
    random.seed(42)  # For reproducible shuffling
    main()
