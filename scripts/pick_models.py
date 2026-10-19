#!/usr/bin/env python3
"""Copies the models the game uses from upstream/packs/ into assets/models/.

Each glTF is copied with every file it references (buffers, textures); each pack's
licence text is copied next to its models. Textures are shrunk to MAX_TEXTURE.
Re-running replaces the copies. Needs macOS (sips).
Usage: python3 -I scripts/pick_models.py
"""
import json
import pathlib
import shutil
import struct
import subprocess

ROOT = pathlib.Path(__file__).resolve().parent.parent
PACKS = ROOT / "upstream" / "packs"
OUT = ROOT / "assets" / "models"
# Textures larger than this are shrunk (the low-detail look does not need more).
MAX_TEXTURE = 1024

UBC = "universal-base-characters"
OUTFITS = "character-outfits-fantasy/Exports/glTF (Godot-Unreal)/Outfits"
HAIR = f"{UBC}/Hairstyles/Rigged to Head Bone/glTF (Godot -Unreal)"
VILLAGE = "medieval-village-megakit/glTF"
NATURE = "stylized-nature-megakit/glTF"
CASTLE = "kenney-castle-kit/Models/GLB format"
PROPS = "fantasy-props-megakit/Exports/glTF"
MONSTERS = "dungeon-monsters-kit-source/Exports/GLB (Godot-Unreal)"
KAYKIT = "kaykit-dungeon-remastered/Assets/gltf"

# (destination folder, pack licence file, source files relative to PACKS)
PICK = [
    ("characters", f"{UBC}/License_Standard.txt", [
        f"{UBC}/Base Characters/Godot - UE/Superhero_Male_FullBody.gltf",
        f"{UBC}/Base Characters/Godot - UE/Superhero_Female_FullBody.gltf",
        f"{HAIR}/Hair_SimpleParted.gltf",
        f"{HAIR}/Hair_Long.gltf",
        f"{HAIR}/Hair_Beard.gltf",
        f"{HAIR}/Eyebrows_Regular.gltf",
        f"{HAIR}/Eyebrows_Female.gltf",
    ]),
    ("outfits", "character-outfits-fantasy/License_Standard.txt", [
        f"{OUTFITS}/Male_Peasant.gltf",
        f"{OUTFITS}/Female_Peasant.gltf",
        f"{OUTFITS}/Male_Ranger.gltf",
        f"{OUTFITS}/Female_Ranger.gltf",
    ]),
    ("animations", "universal-animation-library/License.txt", [
        "universal-animation-library/Unreal-Godot/UAL1_Standard.glb",
    ]),
    ("buildings", "medieval-village-megakit/License_Standard.txt", [f"{VILLAGE}/{n}.gltf" for n in [
        "Wall_Plaster_Straight", "Wall_Plaster_Door_Round", "Wall_Plaster_Window_Wide_Round",
        "Wall_Plaster_WoodGrid", "Wall_UnevenBrick_Straight", "Wall_UnevenBrick_Door_Round",
        "Wall_UnevenBrick_Window_Wide_Round", "Corner_Exterior_Brick", "Corner_Exterior_Wood",
        "Floor_Brick", "Roof_RoundTiles_4x4", "Roof_RoundTiles_4x6", "Roof_RoundTiles_6x6",
        "Roof_RoundTiles_6x8", "Roof_RoundTiles_8x8", "Roof_Tower_RoundTiles", "Prop_Chimney",
        "Prop_WoodenFence_Single", "Prop_MetalFence_Simple",
    ]]),
    ("nature", "stylized-nature-megakit/License_Standard.txt", [f"{NATURE}/{n}.gltf" for n in [
        "CommonTree_1", "CommonTree_2", "CommonTree_3", "CommonTree_4", "CommonTree_5",
        "Pine_1", "Pine_2", "Pine_3", "Pine_4", "Pine_5",
        "DeadTree_1", "DeadTree_2", "DeadTree_3", "TwistedTree_1", "TwistedTree_2",
        "Bush_Common", "Bush_Common_Flowers", "Rock_Medium_1", "Rock_Medium_2", "Rock_Medium_3",
    ]]),
    ("castle", "kenney-castle-kit/License.txt", [f"{CASTLE}/{n}.glb" for n in [
        "wall", "wall-corner", "wall-pillar", "wall-doorway", "wall-narrow", "wall-narrow-corner",
        "wall-narrow-gate", "gate", "metal-gate", "door", "tower-square", "tower-square-arch",
        "tower-hexagon-base", "tower-hexagon-mid", "tower-hexagon-roof",
    ]]),
    ("props", "fantasy-props-megakit/License_Standard.txt", [f"{PROPS}/{n}.gltf" for n in [
        "Chest_Wood", "Key_Gold", "Key_Metal", "Chair_1", "Stool", "Table_Large",
        "Bag", "Pouch_Large", "Bookcase_2", "Crate_Wooden", "Cauldron", "Workbench", "Cabinet", "Barrel",
        "CandleStick_Stand",
    ]]),
    ("kaykit", "kaykit-dungeon-remastered/License.txt", [f"{KAYKIT}/{n}.gltf" for n in ["stairs", "torch", "torch_lit"]]),
    # Paid (Source tier), Quaternius Asset License: fine in this private repo; the
    # licence forbids sharing the files themselves.
    ("monsters", "dungeon-monsters-kit-source/License_Source.txt", [f"{MONSTERS}/{n}.glb" for n in [
        "Hellwarden", "Imp", "Lycan", "Puglin", "Skeleton_A", "Skeleton_B", "Tidebreaker",
    ]]),
]


# Our copies use other textures: the pack's bushes have red (autumn) leaves.
TEXTURE_SWAP = {
    "Bush_Common.gltf": {"Leaves_TwistedTree_C.png": "Leaves_NormalTree_C.png"},
    "Bush_Common_Flowers.gltf": {"Leaves_TwistedTree_C.png": "Leaves_NormalTree_C.png"},
}
# Our copies change their first node: the Kenney door is a block as thick as a wall
# (a dungeon door is about a hand thick); the KayKit stairs start at the model's middle.
NODE = {"door.glb": {"scale": [0.25, 1, 1]}, "stairs.gltf": {"translation": [0, 0, -2]}}


def find(folder: pathlib.Path, uri: str):
    """The file a uri names. Some packs name `T_X_png.png` where only `T_X.png` exists."""
    for name in (uri, uri.replace("_png.png", ".png")):
        if (folder / name).is_file():
            return folder / name
    raise FileNotFoundError(folder / uri)


def unpack_glb(src: pathlib.Path, dest: pathlib.Path, g: dict, data: bytes):
    """Writes a .glb with images inside it as a .gltf, a .bin, and PNG files, so the
    images can be shrunk like any other texture."""
    length = struct.unpack_from("<I", data, 12)[0]
    chunk = 20 + length
    binary = data[chunk + 8:chunk + 8 + struct.unpack_from("<I", data, chunk)[0]]
    views = g["bufferViews"]
    for im in g["images"]:
        v = views[im["bufferView"]]
        name = im.get("name", src.stem).removesuffix(".png") + ".png"
        (dest / name).write_bytes(binary[v.get("byteOffset", 0):v.get("byteOffset", 0) + v["byteLength"]])
        im["uri"] = name
    # Keep only the views the meshes use, packed again, and renumber them.
    dropped = {im.pop("bufferView") for im in g["images"]}
    new_index, out = {}, bytearray()
    for i, v in enumerate(views):
        if i in dropped:
            continue
        out += b"\0" * (-len(out) % 4)
        start = v.get("byteOffset", 0)
        new_index[i] = len(new_index)
        out_offset = len(out)
        out += binary[start:start + v["byteLength"]]
        v["byteOffset"] = out_offset
    g["bufferViews"] = [v for i, v in enumerate(views) if i not in dropped]
    for a in g["accessors"]:
        if "bufferView" in a:
            a["bufferView"] = new_index[a["bufferView"]]
    for im in g["images"]:
        im.pop("mimeType", None)
    g["buffers"] = [{"byteLength": len(out), "uri": f"{src.stem}.bin"}]
    (dest / f"{src.stem}.bin").write_bytes(out)
    (dest / f"{src.stem}.gltf").write_text(json.dumps(g))


def copy_model(src: pathlib.Path, dest: pathlib.Path):
    """Copies a model and the files it names; fixes broken names in a .gltf copy.
    A .glb keeps its JSON as is, so its outside files keep their relative paths,
    unless it holds its images: then it becomes a .gltf (see `unpack_glb`)."""
    if src.suffix.lower() == ".glb":
        data = src.read_bytes()
        length = struct.unpack_from("<I", data, 12)[0]
        g = json.loads(data[20:20 + length])
        if any("bufferView" in im for im in g.get("images", [])):
            unpack_glb(src, dest, g, data)
            return
        if src.name in NODE:
            g["nodes"][0].update(NODE[src.name])
            js = json.dumps(g).encode()
            js += b" " * (-len(js) % 4)
            rest = data[20 + length:]
            (dest / src.name).write_bytes(data[:4] + struct.pack("<III", 2, 20 + len(js) + len(rest), len(js))
                                          + data[16:20] + js + rest)
        else:
            shutil.copy2(src, dest / src.name)
        for item in g.get("buffers", []) + g.get("images", []):
            uri = item.get("uri")
            if uri and not uri.startswith("data:"):
                (dest / uri).parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(src.parent / uri, dest / uri)
        return
    g = json.loads(src.read_text())
    for item in g.get("buffers", []) + g.get("images", []):
        uri = item.get("uri")
        if uri and not uri.startswith("data:"):
            uri = TEXTURE_SWAP.get(src.name, {}).get(uri, uri)
            f = find(src.parent, uri)
            shutil.copy2(f, dest / f.name)
            item["uri"] = f.name
    if src.name in NODE:
        g["nodes"][0].update(NODE[src.name])
    (dest / src.name).write_text(json.dumps(g))


def main():
    total = 0
    for folder, licence, files in PICK:
        dest = OUT / folder
        if dest.exists():
            shutil.rmtree(dest)
        dest.mkdir(parents=True)
        shutil.copy2(PACKS / licence, dest / "LICENSE.txt")
        for rel in files:
            copy_model(PACKS / rel, dest)
        pngs = [str(f) for f in dest.rglob("*.png")]
        if pngs:  # macOS sips; only shrinks, never enlarges
            subprocess.run(["sips", "-Z", str(MAX_TEXTURE), *pngs], check=True, capture_output=True)
        size = sum(f.stat().st_size for f in dest.rglob("*") if f.is_file())
        total += size
        print(f"{folder}: {len(files)} models, {size / 1e6:.1f} MB")
    print(f"total {total / 1e6:.1f} MB")


if __name__ == "__main__":
    main()
