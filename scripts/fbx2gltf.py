"""Converts every FBX in a folder to glTF (.gltf + .bin, with its animations), for the
Quaternius packs that ship FBX only. Their FBX materials say fully transparent, so the
copies are made solid.

    /Applications/Blender.app/Contents/MacOS/Blender -b --python scripts/fbx2gltf.py -- <fbx folder> <glTF folder>
"""
import glob
import json
import os
import sys

import bpy

src, dst = sys.argv[sys.argv.index("--") + 1:][:2]
os.makedirs(dst, exist_ok=True)
for f in sorted(glob.glob(os.path.join(src, "*.fbx"))):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=f)
    out = os.path.join(dst, os.path.splitext(os.path.basename(f))[0] + ".gltf")
    bpy.ops.export_scene.gltf(filepath=out, export_format="GLTF_SEPARATE", export_animations=True, export_animation_mode="ACTIONS")
    with open(out) as fh:
        g = json.load(fh)
    for m in g.get("materials", []):
        pbr = m.setdefault("pbrMetallicRoughness", {})
        pbr["baseColorFactor"] = pbr.get("baseColorFactor", [1, 1, 1, 1])[:3] + [1]
        m.pop("alphaMode", None)
        m.pop("alphaCutoff", None)
    with open(out, "w") as fh:
        json.dump(g, fh)
    print("CONVERTED", out)
