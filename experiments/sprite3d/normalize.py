#!/usr/bin/env python3
"""Fits an AI-made model over its picture, like the picture card:

    normalize.py RAW.glb PICTURE.png OUT.glb

Scaled evenly so it fits the picture's opaque part (1/32 unit per pixel), standing
on y = 0, centred on that part left to right and front to back (x = 0 is the image's
middle, as in voxel.py). Needs trimesh (the venv).
"""
import sys

import numpy as np
import trimesh
from PIL import Image

PX = 1 / 32

raw, picture, out = sys.argv[1:4]
alpha = np.asarray(Image.open(picture).convert("RGBA"))[..., 3] >= 128
h, w = alpha.shape
rows, cols = np.nonzero(alpha)
# Opaque box in model units: x from the image middle, y up from the ground.
x0, x1 = (cols.min() - w / 2) * PX, (cols.max() + 1 - w / 2) * PX
y0, y1 = 0.0, (rows.max() + 1 - rows.min()) * PX

scene = trimesh.load(raw, force="scene")
lo, hi = scene.bounds
size = hi - lo
s = min((x1 - x0) / size[0], (y1 - y0) / size[1])
centre = (lo + hi) / 2
m = np.eye(4)
m[:3, :3] *= s
m[:3, 3] = [(x0 + x1) / 2 - centre[0] * s, y0 - lo[1] * s, -centre[2] * s]
scene.apply_transform(m)
scene.export(out)
