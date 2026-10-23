#!/usr/bin/env python3
"""Deterministic sprite -> 3D: every opaque pixel becomes a column of voxels.

    voxel.py extrude|inflate IN.png OUT.glb

- extrude: a slab DEPTH pixels thick (the "Minecraft item" look).
- inflate: thicker in the middle, thin at the edges (depth grows with the square
  root of the distance to the outline), so round things look round.

One pixel is 1/32 unit, like the game's picture cards; the image's middle is x = 0
and its lowest opaque row stands on y = 0 (the ground). Only faces
between a filled and an empty voxel are kept. Vertex colours, no texture. Needs
numpy and Pillow.
"""
import json
import struct
import sys

import numpy as np
from PIL import Image

PX = 1 / 32
DEPTH = 4
INFLATE = 3.0  # depth (each side) at distance d from the outline: INFLATE * sqrt(d)


def edge_distance(mask):
    """Chessboard distance from each opaque pixel to the nearest transparent one."""
    d = np.where(mask, 10_000, 0)
    p = np.pad(d, 1)
    for _ in range(max(mask.shape)):
        q = np.minimum.reduce([p[1:-1, 1:-1], p[:-2, 1:-1] + 1, p[2:, 1:-1] + 1, p[1:-1, :-2] + 1, p[1:-1, 2:] + 1])
        if (q == p[1:-1, 1:-1]).all():
            break
        p[1:-1, 1:-1] = q
    return p[1:-1, 1:-1]


def grid(rgba, mode):
    """Bool grid [y, x, z] and the depth layers each pixel fills."""
    mask = rgba[..., 3] >= 128
    if mode == "extrude":
        half = np.where(mask, DEPTH / 2, 0)
    else:
        half = np.where(mask, np.maximum(1, np.round(INFLATE * np.sqrt(edge_distance(mask)))), 0)
    zmax = int(half.max())
    z = np.arange(-zmax, zmax) + 0.5  # voxel centres
    return np.abs(z)[None, None, :] < half[..., None], zmax


def mesh(rgba, mode):
    """Turns a sprite into coloured voxel-box faces (positions, normals, colours, indices), standing on the ground."""
    filled, zmax = grid(rgba, mode)
    h, w, _ = filled.shape
    lin = (rgba[..., :3] / 255.0) ** 2.2  # glTF vertex colours are linear
    pos, nrm, col = [], [], []
    p = np.pad(filled, 1)
    # (axis, sign, the 4 corners of that face of a unit cube at the origin)
    faces = [
        ((0, 0, 1), [(0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)]),
        ((0, 0, -1), [(1, 0, 0), (0, 0, 0), (0, 1, 0), (1, 1, 0)]),
        ((1, 0, 0), [(1, 0, 1), (1, 0, 0), (1, 1, 0), (1, 1, 1)]),
        ((-1, 0, 0), [(0, 0, 0), (0, 0, 1), (0, 1, 1), (0, 1, 0)]),
        ((0, 1, 0), [(0, 1, 1), (1, 1, 1), (1, 1, 0), (0, 1, 0)]),
        ((0, -1, 0), [(0, 0, 0), (1, 0, 0), (1, 0, 1), (0, 0, 1)]),
    ]
    for (nx, ny, nz), corners in faces:
        # Neighbour in the grid [row, col, layer]: +y in space is one row up.
        nb = p[1 - ny : 1 - ny + h, 1 + nx : 1 + nx + w, 1 + nz : 1 + nz + 2 * zmax]
        ys, xs, zs = np.nonzero(filled & ~nb)
        base = np.stack([xs - w / 2, h - 1 - ys, zs - zmax], axis=1).astype(np.float32)
        for c in corners:
            pos.append(base + np.array(c, np.float32))
        nrm.append(np.tile(np.array([nx, ny, nz], np.float32), (len(xs), 1)))
        col.append(lin[ys, xs].astype(np.float32))
    # Interleave so each face's 4 corners are consecutive.
    quads = [np.stack(group, axis=1) for group in zip(*[iter(pos)] * 4)]
    positions = np.concatenate(quads).reshape(-1, 3) * PX
    positions[:, 1] -= positions[:, 1].min()
    normals = np.repeat(np.concatenate(nrm), 4, axis=0)
    colours = np.repeat(np.concatenate(col), 4, axis=0)
    n = len(positions) // 4
    idx = (np.arange(n, dtype=np.uint32)[:, None] * 4 + np.array([0, 1, 2, 0, 2, 3], np.uint32)).ravel()
    return positions, normals, colours, idx


def write_glb(path, positions, normals, colours, idx):
    """Saves the mesh arrays as a single binary .glb file."""
    blobs = [positions.astype("<f4").tobytes(), normals.astype("<f4").tobytes(), colours.astype("<f4").tobytes(), idx.astype("<u4").tobytes()]
    views, offset = [], 0
    for b in blobs:
        views.append({"buffer": 0, "byteOffset": offset, "byteLength": len(b)})
        offset += len(b)
    n = len(positions)
    gltf = {
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1, "COLOR_0": 2}, "indices": 3, "material": 0}]}],
        "materials": [{"pbrMetallicRoughness": {"metallicFactor": 0.0, "roughnessFactor": 0.9}}],
        "buffers": [{"byteLength": offset}],
        "bufferViews": views,
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": n, "type": "VEC3", "min": positions.min(0).tolist(), "max": positions.max(0).tolist()},
            {"bufferView": 1, "componentType": 5126, "count": n, "type": "VEC3"},
            {"bufferView": 2, "componentType": 5126, "count": n, "type": "VEC3"},
            {"bufferView": 3, "componentType": 5125, "count": len(idx), "type": "SCALAR"},
        ],
    }
    js = json.dumps(gltf).encode()
    js += b" " * (-len(js) % 4)
    bin_ = b"".join(blobs)
    with open(path, "wb") as f:
        f.write(struct.pack("<III", 0x46546C67, 2, 12 + 8 + len(js) + 8 + len(bin_)))
        f.write(struct.pack("<II", len(js), 0x4E4F534A) + js)
        f.write(struct.pack("<II", len(bin_), 0x004E4942) + bin_)


def demo():
    # A 2x1 opaque strip, extruded: 1 box of 2x1x4 voxels -> 2*(2*1 + 2*4 + 1*4) = 28 faces.
    """Self-test (`voxel.py test`): checks the face count, ground level and inflate depth."""
    rgba = np.zeros((3, 4, 4), np.uint8)
    rgba[1, 1:3] = 255
    p, _, _, idx = mesh(rgba, "extrude")
    assert len(idx) == 28 * 6, len(idx)
    assert p[:, 1].min() == 0 and abs(p[:, 1].max() - 1 * PX) < 1e-6  # stands on the ground
    # Inflate: a 5x5 square's middle is deeper than its edge.
    rgba = np.zeros((7, 7, 4), np.uint8)
    rgba[1:6, 1:6] = 255
    d = edge_distance(rgba[..., 3] >= 128)
    assert d[3, 3] == 3 and d[1, 1] == 1 and d[0, 0] == 0
    print("ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["test"]:
        demo()
    else:
        mode, src, out = sys.argv[1:4]
        write_glb(out, *mesh(np.asarray(Image.open(src).convert("RGBA")), mode))
