#!/usr/bin/env python3
"""Generates the seamless background tiles for CharSheet's Stone and Parchment themes.

Usage: python3 tools/make_textures.py crates/charsheet/theme
Needs numpy and Pillow. The output is deterministic, so rerunning it changes nothing.
"""

import sys
from pathlib import Path

import numpy as np
from PIL import Image

SIZE = 256


def tileable_noise(rng, scale):
    """Noise that wraps at the edges: white noise low-pass filtered in frequency space."""
    white = rng.standard_normal((SIZE, SIZE))
    fy = np.fft.fftfreq(SIZE)[:, None]
    fx = np.fft.fftfreq(SIZE)[None, :]
    falloff = np.exp(-(fx**2 + fy**2) * (SIZE / scale) ** 2)
    field = np.real(np.fft.ifft2(np.fft.fft2(white) * falloff))
    return (field - field.mean()) / field.std()


def texture(base, seed, mottle, grain):
    rng = np.random.default_rng(seed)
    shade = (
        tileable_noise(rng, 6) * 0.6 + tileable_noise(rng, 24) * 0.3 + tileable_noise(rng, 80) * 0.1
    ) * mottle + rng.standard_normal((SIZE, SIZE)) * grain
    rgb = np.clip(np.array(base, dtype=float)[None, None, :] + shade[:, :, None], 0, 255)
    return Image.fromarray(rgb.astype(np.uint8), "RGB")


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    texture((230, 230, 227), seed=5, mottle=5.0, grain=3.5).save(out / "stone.png", optimize=True)
    texture((238, 229, 206), seed=7, mottle=6.0, grain=2.5).save(out / "parchment.png", optimize=True)


if __name__ == "__main__":
    main()
