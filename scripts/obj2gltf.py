#!/usr/bin/env python3
"""Converts simple Wavefront OBJ files (flat colours from their .mtl, no textures),
like Quaternius' Medieval Weapons Pack, to glTF that Bevy loads.

    scripts/obj2gltf.py OUT_DIR file.obj...

Each .obj becomes OUT_DIR/<name>.gltf + <name>.bin: one mesh, one primitive per
material, the material's Kd as its base colour. Stdlib only.
"""
import json
import os
import struct
import sys


def read_mtl(path):
    colours, name = {}, None
    if not os.path.exists(path):
        return colours
    for line in open(path):
        w = line.split()
        if w[:1] == ["newmtl"]:
            name = w[1]
        elif w[:1] == ["Kd"] and name:
            colours[name] = [float(x) for x in w[1:4]]
    return colours


def read_face(corners_text, verts, index, tris):
    """Adds one face's new vertices and fan triangles to a material's group."""
    corners = []
    for c in corners_text:
        parts = c.split("/")
        key = (int(parts[0]) - 1, int(parts[2]) - 1 if len(parts) > 2 and parts[2] else -1)
        if key not in index:
            index[key] = len(verts)
            verts.append(key)
        corners.append(index[key])
    for i in range(1, len(corners) - 1):  # fan: faces are convex quads and ngons
        tris.append((corners[0], corners[i], corners[i + 1]))


def read_obj(obj_path):
    """Parses an OBJ into material colours, positions, normals and per-material groups."""
    folder = os.path.dirname(obj_path)
    colours, pos, nrm = {}, [], []
    groups = {}  # material -> (vertices [(p, n)], index of each vertex, triangles)
    mat = None
    for line in open(obj_path):
        w = line.split()
        if not w:
            continue
        if w[0] == "mtllib":
            colours.update(read_mtl(os.path.join(folder, " ".join(w[1:]))))
        elif w[0] == "v":
            pos.append([float(x) for x in w[1:4]])
        elif w[0] == "vn":
            nrm.append([float(x) for x in w[1:4]])
        elif w[0] == "usemtl":
            mat = w[1]
        elif w[0] == "f":
            read_face(w[1:], *groups.setdefault(mat, ([], {}, [])))
    return colours, pos, nrm, groups


class Buffer:
    """The glTF binary blob plus the bufferViews and accessors that slice it."""

    def __init__(self):
        self.blob, self.views, self.accessors = bytearray(), [], []

    def add(self, data, target, fields):
        """Appends data as a 4-byte aligned view and accessor; returns the accessor index."""
        while len(self.blob) % 4:
            self.blob.append(0)
        self.views.append({"buffer": 0, "byteOffset": len(self.blob), "byteLength": len(data), "target": target})
        self.blob.extend(data)
        self.accessors.append({"bufferView": len(self.views) - 1, **fields})
        return len(self.accessors) - 1


def add_geometry(buf, p, n, tris):
    """Writes positions, normals and indices to the buffer; returns their accessor indices."""
    lo = [min(c[i] for c in p) for i in range(3)]
    hi = [max(c[i] for c in p) for i in range(3)]
    pa = buf.add(b"".join(struct.pack("<3f", *c) for c in p), 34962, {
        "componentType": 5126, "count": len(p), "type": "VEC3", "min": lo, "max": hi})
    na = buf.add(b"".join(struct.pack("<3f", *c) for c in n), 34962, {"componentType": 5126, "count": len(n), "type": "VEC3"})
    ia = buf.add(b"".join(struct.pack("<3I", *t) for t in tris), 34963, {"componentType": 5125, "count": len(tris) * 3, "type": "SCALAR"})
    return pa, na, ia


def build_primitives(buf, pos, nrm, groups, colours):
    """Builds one primitive and one material per OBJ material group."""
    prims, materials = [], []
    for m, (verts, _, tris) in groups.items():
        p = [pos[v] for v, _ in verts]
        n = [nrm[k] if k >= 0 else [0.0, 1.0, 0.0] for _, k in verts]
        pa, na, ia = add_geometry(buf, p, n, tris)
        r, g, b = colours.get(m, [0.8, 0.8, 0.8])
        materials.append({"name": m or "Default", "pbrMetallicRoughness": {
            "baseColorFactor": [r, g, b, 1.0], "metallicFactor": 0.0, "roughnessFactor": 0.6}})
        prims.append({"attributes": {"POSITION": pa, "NORMAL": na}, "indices": ia, "material": len(materials) - 1})
    return prims, materials


def write_gltf(out_dir, base, buf, prims, materials):
    """Writes the .bin blob and the .gltf JSON that describes it."""
    gltf = {
        "asset": {"version": "2.0", "generator": "Crossfir3d scripts/obj2gltf.py"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"name": base, "mesh": 0}],
        "meshes": [{"name": base, "primitives": prims}],
        "materials": materials,
        "accessors": buf.accessors,
        "bufferViews": buf.views,
        "buffers": [{"uri": base + ".bin", "byteLength": len(buf.blob)}],
    }
    with open(os.path.join(out_dir, base + ".bin"), "wb") as f:
        f.write(buf.blob)
    with open(os.path.join(out_dir, base + ".gltf"), "w") as f:
        json.dump(gltf, f, indent=1)


def convert(obj_path, out_dir):
    """Converts one OBJ file to a .gltf and .bin in out_dir."""
    base = os.path.splitext(os.path.basename(obj_path))[0]
    colours, pos, nrm, groups = read_obj(obj_path)
    buf = Buffer()
    prims, materials = build_primitives(buf, pos, nrm, groups, colours)
    write_gltf(out_dir, base, buf, prims, materials)


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    os.makedirs(sys.argv[1], exist_ok=True)
    for path in sys.argv[2:]:
        convert(path, sys.argv[1])
