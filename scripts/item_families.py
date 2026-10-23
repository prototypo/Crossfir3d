#!/usr/bin/env python3
"""Points small items in assets/models.txt at our own family recipes (`sword:r,g,b`, ...),
coloured from each item's picture. Items no family fits keep their voxel model.

    experiments/sprite3d/.venv/bin/python -I scripts/item_families.py

Only plain lines `name model auto/name.glb 0.5 0 0` change. Needs Pillow and numpy.
"""
import csv
import colorsys
import os
import re

import numpy as np
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MODELS = os.path.join(ROOT, "assets/models.txt")
ARCH = os.path.join(ROOT, "upstream/crossfire-arch")
CATALOG = os.path.join(ROOT, "experiments/sprite3d/catalog.tsv")

# First match wins.
FAMILIES = [
    ("dagger", r"dagger|knife|^sai_|^kama_|^stake$"),
    ("arrow", r"^arrow|^bolt(_|$)"),
    ("bow", r"bow$|bow_bone$|^crossbow"),
    ("sword", r"sword|sabre|scimitar|katana|rapier|falchion|excalibur|masamune|blade|brand$|reaver|bring$|demonbane"
              r"|dragonslay|^gram$|^defender$|^wds$|^uk_sd$|harakiri|idaten|taifu|holyave"),
    ("hammer", r"hamme|mjoellnir|bonecrush"),
    ("axe", r"axe"),
    ("mace", r"mace|mornstar|flail"),
    ("club", r"club|cudgel"),
    ("spear", r"spear|trident"),
    ("staff", r"staff|^rod_|^wand$"),
    ("ring", r"^ring"),
    ("amulet", r"^amulet"),
    ("potion", r"^potion|^vial_|^water_"),
    ("shield", r"shield|shie$"),
    ("helmet", r"helm"),
    ("coin", r"coin$"),
    ("gem", r"diamond|ruby|emerald|sapphire|amethyst|pearl|opal|garnet|aquamarine|moonstone|onyx|turquoise|jetstone"
            r"|bloodstone|crystal"),
    ("ingot", r"bar$"),
    ("fruit", r"^apple$|^orange$|^pear$|^tomato"),
]
# The gem, the liquid, the stone on top takes the colour; else the whole thing does.
BRIGHT_PART = {"ring", "amulet", "potion", "gem", "fruit", "staff"}
LINE = re.compile(r"^(\S+) model auto/(\S+)\.glb 0\.5 0 0$")


def family(name):
    return next((f for f, rx in FAMILIES if re.search(rx, name)), None)


def colour(path, bright_part):
    """Returns the average "r,g,b" of a picture's visible non-dark pixels, or of its most saturated ones for bright parts."""
    px = np.asarray(Image.open(path).convert("RGBA"), dtype=float) / 255
    px = px[px[..., 3] > 0.5][:, :3]
    hsv = np.array([colorsys.rgb_to_hsv(*p) for p in px])
    px, hsv = px[hsv[:, 2] > 0.25], hsv[hsv[:, 2] > 0.25]  # no black outlines
    if len(px) == 0:
        return None
    if bright_part:
        keep = hsv[:, 1] >= np.quantile(hsv[:, 1], 0.7)
        px = px[keep]
    return ",".join(f"{v:.2f}" for v in px.mean(axis=0))


def main():
    """Rewrites the model list so items in a family use a coloured recipe taken from their picture."""
    pictures = {}
    for row in csv.DictReader(open(CATALOG), delimiter="\t"):
        thing = row["picture"].split(".")[0]
        pictures[thing] = min(pictures.get(thing, row["file"]), row["file"])
    out, changed = [], 0
    for line in open(MODELS).read().splitlines():
        m = LINE.match(line)
        kind = m and family(m.group(2))
        rgb = kind and m.group(2) in pictures and colour(os.path.join(ARCH, pictures[m.group(2)]), kind in BRIGHT_PART)
        if rgb:
            line, changed = f"{m.group(1)} building {kind}:{rgb}", changed + 1
        out.append(line)
    open(MODELS, "w").write("\n".join(out) + "\n")
    print(f"{changed} items now use a family recipe")


if __name__ == "__main__":
    main()
