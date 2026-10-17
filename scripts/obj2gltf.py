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


def convert(obj_path, out_dir):
    base = os.path.splitext(os.path.basename(obj_path))[0]
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
            verts, index, tris = groups.setdefault(mat, ([], {}, []))
            corners = []
            for c in w[1:]:
                parts = c.split("/")
                key = (int(parts[0]) - 1, int(parts[2]) - 1 if len(parts) > 2 and parts[2] else -1)
                if key not in index:
                    index[key] = len(verts)
                    verts.append(key)
                corners.append(index[key])
            for i in range(1, len(corners) - 1):  # fan: faces are convex quads and ngons
                tris.append((corners[0], corners[i], corners[i + 1]))

    blob, views, accessors, prims, materials = bytearray(), [], [], [], []

    def add(data, target, count, kind, comp, extra=None):
        while len(blob) % 4:
            blob.append(0)
        views.append({"buffer": 0, "byteOffset": len(blob), "byteLength": len(data), "target": target})
        blob.extend(data)
        acc = {"bufferView": len(views) - 1, "componentType": comp, "count": count, "type": kind}
        acc.update(extra or {})
        accessors.append(acc)
        return len(accessors) - 1

    for m, (verts, _, tris) in groups.items():
        p = [pos[v] for v, _ in verts]
        n = [nrm[k] if k >= 0 else [0.0, 1.0, 0.0] for _, k in verts]
        lo = [min(c[i] for c in p) for i in range(3)]
        hi = [max(c[i] for c in p) for i in range(3)]
        pa = add(b"".join(struct.pack("<3f", *c) for c in p), 34962, len(p), "VEC3", 5126, {"min": lo, "max": hi})
        na = add(b"".join(struct.pack("<3f", *c) for c in n), 34962, len(n), "VEC3", 5126)
        ia = add(b"".join(struct.pack("<3I", *t) for t in tris), 34963, len(tris) * 3, "SCALAR", 5125)
        r, g, b = colours.get(m, [0.8, 0.8, 0.8])
        materials.append({"name": m or "Default", "pbrMetallicRoughness": {
            "baseColorFactor": [r, g, b, 1.0], "metallicFactor": 0.0, "roughnessFactor": 0.6}})
        prims.append({"attributes": {"POSITION": pa, "NORMAL": na}, "indices": ia, "material": len(materials) - 1})

    gltf = {
        "asset": {"version": "2.0", "generator": "MacCrossfire scripts/obj2gltf.py"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"name": base, "mesh": 0}],
        "meshes": [{"name": base, "primitives": prims}],
        "materials": materials,
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"uri": base + ".bin", "byteLength": len(blob)}],
    }
    with open(os.path.join(out_dir, base + ".bin"), "wb") as f:
        f.write(blob)
    with open(os.path.join(out_dir, base + ".gltf"), "w") as f:
        json.dump(gltf, f, indent=1)


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    os.makedirs(sys.argv[1], exist_ok=True)
    for path in sys.argv[2:]:
        convert(path, sys.argv[1])
