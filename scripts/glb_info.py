#!/usr/bin/env python3
"""Prints what a .glb/.gltf holds: meshes, nodes, animations, materials, extensions.

Usage: python3 -I scripts/glb_info.py FILE [FILE...]
Standard library only (run with -I so nothing next to the model files is imported).
"""
import json
import struct
import sys


def read_json(path):
    with open(path, "rb") as f:
        data = f.read()
    if data[:4] == b"glTF":  # binary: 12-byte header, then chunks
        length, kind = struct.unpack_from("<II", data, 12)
        if kind != 0x4E4F534A:  # "JSON"
            raise ValueError("first chunk is not JSON")
        return json.loads(data[20:20 + length])
    return json.loads(data)


def main(paths):
    for path in paths:
        try:
            g = read_json(path)
        except Exception as e:  # report and go on with the next file
            print(f"== {path}: cannot read ({e})")
            continue
        anims = g.get("animations", [])
        print(f"== {path}")
        print(f"   meshes {len(g.get('meshes', []))}, nodes {len(g.get('nodes', []))}, "
              f"skins {len(g.get('skins', []))}, materials {len(g.get('materials', []))}, "
              f"images {len(g.get('images', []))}")
        used = g.get("extensionsUsed", [])
        if used:
            print(f"   extensions: {', '.join(used)}")
        if anims:
            print(f"   animations ({len(anims)}): {', '.join(a.get('name', '?') for a in anims)}")


if __name__ == "__main__":
    main(sys.argv[1:])
