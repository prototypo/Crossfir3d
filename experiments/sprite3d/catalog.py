#!/usr/bin/env python3
"""Sorts every Crossfire picture into the 3D technique most likely to work for it.

    python3 -I experiments/sprite3d/catalog.py          # writes catalog.tsv, CATALOG.md
    python3 -I experiments/sprite3d/catalog.py samples  # also one sample sheet per group
    python3 -I experiments/sprite3d/catalog.py test     # the pilot's verdicts still hold

Groups (see RESULTS.md for why):
  auto      thin or simple items: Hunyuan3D + clean-up, inflated voxels as fallback
  existing  already 3D in the game (assets/models.txt, walls, player characters)
  rigged    creatures: rigged kit models, like players and monsters today
  scratch   boxy, big, worn, or furniture-like things: model from scratch
  flat      ground, floor marks, spell effects: stay pictures
  skip      never seen as a thing in the world (UI icons, invisible system objects,
            things to model that no object or map shows)
  review    no rule fits well; a person decides

Each picture is linked to the objects (.arc) that show it, directly (`face`) or in an
animation (.face / inline `anim`). The first rule that matches wins. Needs Pillow.
"""
import os
import random
import re
import subprocess
import sys
from collections import Counter, defaultdict

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
ARCH = os.path.join(ROOT, "upstream/crossfire-arch")
MAPS = os.path.join(ROOT, "upstream/crossfire-maps")
SHOTS = os.path.join(HERE, "shots")

# Crossfire object types (crossfire-server/include/object.h).
TYPES = dict(
    PLAYER=1, TRANSPORT=2, ROD=3, TREASURE=4, POTION=5, FOOD=6, POISON=7, BOOK=8, CLOCK=9, DRAGON_FOCUS=10,
    ARROW=13, BOW=14, WEAPON=15, ARMOUR=16, PEDESTAL=17, ALTAR=18, LOCKED_DOOR=20, SPECIAL_KEY=21, MAP=22,
    DOOR=23, KEY=24, TIMED_GATE=26, TRIGGER=27, GRIMREAPER=28, MAGIC_EAR=29, TRIGGER_BUTTON=30,
    TRIGGER_ALTAR=31, TRIGGER_PEDESTAL=32, SHIELD=33, HELMET=34, MONEY=36, CLASS=37, AMULET=39,
    PLAYERMOVER=40, TELEPORTER=41, CREATOR=42, SKILL=43, EARTHWALL=45, GOLEM=46, THROWN_OBJ=48, BLINDNESS=49,
    GOD=50, DETECTOR=51, TRIGGER_MARKER=52, DEAD_OBJECT=53, DRINK=54, MARKER=55, HOLY_ALTAR=56,
    PLAYER_CHANGER=57, BATTLEGROUND=58, PEACEMAKER=59, GEM=60, FIREWALL=62, CHECK_INV=64, MOOD_FLOOR=65,
    EXIT=66, ENCOUNTER=67, SHOP_FLOOR=68, SHOP_MAT=69, RING=70, FLOOR=71, FLESH=72, INORGANIC=73,
    SKILL_TOOL=74, LIGHTER=75, WALL=77, MISC_OBJECT=79, MONSTER=80, LAMP=82, DUPLICATOR=83, SPELLBOOK=85,
    CLOAK=87, SPINNER=90, GATE=91, BUTTON=92, CF_HANDLE=93, HOLE=94, TRAPDOOR=95, SIGN=98, BOOTS=99,
    GLOVES=100, SPELL=101, SPELL_EFFECT=102, CONVERTER=103, BRACERS=104, POISONING=105, SAVEBED=106,
    WAND=109, SCROLL=111, DIRECTOR=112, GIRDLE=113, FORCE=114, POTION_RESIST_EFFECT=115,
    EVENT_CONNECTOR=116, CLOSE_CON=121, CONTAINER=122, ARMOUR_IMPROVER=123, WEAPON_IMPROVER=124,
    SKILLSCROLL=130, DEEP_SWAMP=138, IDENTIFY_ALTAR=139, SHOP_INVENTORY=150, RUNE=154, TRAP=155,
    POWER_CRYSTAL=156, CORPSE=157, DISEASE=158, SYMPTOM=159, BUILDER=160, MATERIAL=161, MIMIC=162,
    LIGHTABLE=163,
)
NAME = {v: k for k, v in TYPES.items()}


def of(*names):
    return {TYPES[n] for n in names}


# (group, types, why). Checked in this order, against every object showing the
# picture: a picture a monster and a god share is a creature, not a system object.
BY_TYPE = [
    ("rigged", of("PLAYER", "MONSTER", "GOLEM", "GRIMREAPER", "MIMIC"), "a creature"),
    ("flat", of("FLOOR", "SHOP_FLOOR", "SHOP_MAT", "DEEP_SWAMP", "MOOD_FLOOR", "SPELL_EFFECT", "FIREWALL",
                "PLAYERMOVER", "DIRECTOR", "SPINNER", "BATTLEGROUND", "RUNE", "HOLE"), "on the ground or an effect"),
    ("scratch", of("ARMOUR", "CLOAK"), "worn on the body: must fit the 3D character"),
    ("auto", of("WEAPON", "BOW", "ARROW", "SHIELD", "HELMET", "RING", "AMULET", "GEM", "MONEY", "KEY",
                "SPECIAL_KEY", "WAND", "ROD", "POTION", "DRINK", "FOOD", "POISON", "FLESH", "SKILL_TOOL",
                "LIGHTER", "POWER_CRYSTAL", "THROWN_OBJ", "BOOTS", "GLOVES", "BRACERS", "GIRDLE", "MATERIAL",
                "TRAP", "BUILDER", "DRAGON_FOCUS"), "a small hand-held item"),
    ("scratch", of("BOOK", "SPELLBOOK", "SCROLL", "SKILLSCROLL", "ARMOUR_IMPROVER", "WEAPON_IMPROVER",
                   "CONTAINER", "ALTAR", "HOLY_ALTAR", "IDENTIFY_ALTAR", "TRIGGER_ALTAR", "PEDESTAL",
                   "TRIGGER_PEDESTAL", "TRANSPORT", "SIGN", "DOOR", "LOCKED_DOOR", "GATE", "TIMED_GATE",
                   "CF_HANDLE", "BUTTON", "TRIGGER_BUTTON", "TRIGGER", "LAMP", "LIGHTABLE", "CLOCK", "SAVEBED",
                   "CONVERTER", "EARTHWALL", "CORPSE", "MAP", "EXIT", "TELEPORTER", "TRAPDOOR"),
     "boxy, drawn from an angle, or furniture"),
    ("skip", of("SPELL", "SKILL", "FORCE", "DISEASE", "SYMPTOM", "MARKER", "TRIGGER_MARKER",
                "MAGIC_EAR", "CLASS", "GOD", "POTION_RESIST_EFFECT", "POISONING", "BLINDNESS", "DUPLICATOR",
                "CREATOR", "DETECTOR", "CHECK_INV", "SHOP_INVENTORY",
                "PLAYER_CHANGER", "PEACEMAKER", "DEAD_OBJECT"), "a UI icon or system object"),
]

# Folders whose pictures are all one thing, whatever their objects' types say.
FOLDER_FIRST = {
    "ground": ("flat", "ground and its smoothing edges"),
    "floor": ("flat", "floor"),
    "road": ("flat", "road"),
    "river": ("flat", "river"),
    "mapbuilding": ("skip", "a building-kit icon (shows what it builds)"),
    "random": ("skip", "a placeholder for a random item"),
}

# For pictures no object uses, or with no telling type: the top folder decides.
# (Encounter, treasure, closed-container and event objects reuse other pictures, so
# their type tells nothing.)
BY_FOLDER = {
    "existing": {"player"},
    "flat": {"ground", "floor", "road", "river", "spell", "attackmess", "planes"},
    "skip": {"system", "skills", "disease", "gods"},
    "rigged": {"monster"},
    "auto": {"weapon", "jewel", "talisman", "potion", "food", "flesh", "inorganic", "crafting"},
    "scratch": {"indoor", "readable", "door", "connect", "transport", "light", "shop", "armour", "construct",
                "mapbuilding", "exit", "misc", "artifacts", "traps", "random", "wall"},
}


def read_lines(path):
    with open(path, encoding="latin-1") as f:
        return [line.strip() for line in f]


def anim_frames(lines, i):
    """Frame lines from index i up to the `mina` line; returns (frames, index of that line)."""
    frames = []
    while i < len(lines) and lines[i] != "mina":
        if not lines[i].startswith("facings"):
            frames.append(lines[i])
        i += 1
    return frames, i


def parse_face(path, anims):
    """Add every `animation` block of one .face file to anims."""
    lines, i = read_lines(path), 0
    while i < len(lines):
        if lines[i].startswith("animation "):
            name = lines[i].split(None, 1)[1]
            anims[name], i = anim_frames(lines, i + 1)
        i += 1


def arc_line(line, obj, folder, objects):
    """Handle one .arc line outside anim/msg blocks; returns (obj, in_anim, in_msg)."""
    if line.startswith("Object "):
        obj = {"name": line.split(None, 1)[1], "folder": folder, "frames": [], "tail": False}
        objects.append(obj)
    elif line == "More":
        obj = None  # tail parts of a big object repeat its keys
    elif line == "anim":
        return obj, True, False
    elif line == "msg":
        return obj, False, True
    elif obj is not None and " " in line:
        k, v = line.split(None, 1)
        obj.setdefault(k, v)
    return obj, False, False


def parse_arc(path, folder, objects):
    """Append every object of one .arc file to objects."""
    obj, in_anim, in_msg = None, False, False
    for line in read_lines(path):
        if in_msg:
            in_msg = line != "endmsg"
        elif in_anim:
            if line == "mina":
                in_anim = False
            elif obj and not line.startswith("facings"):
                obj["frames"].append(line)
        else:
            obj, in_anim, in_msg = arc_line(line, obj, folder, objects)


def parse_arch():
    """Objects (dicts) and animations (name -> frames) from every .arc and .face file."""
    objects, anims = [], {}
    for dirpath, _, files in os.walk(ARCH):
        for fn in files:
            path = os.path.join(dirpath, fn)
            if fn.endswith(".face"):
                parse_face(path, anims)
            elif fn.endswith(".arc"):
                parse_arc(path, os.path.relpath(dirpath, ARCH), objects)
    return objects, anims


def model_map():
    """`assets/models.txt` as (exact, prefixes): name -> kind; same rules as models.rs."""
    exact, prefixes = {}, {}
    for line in read_lines(os.path.join(ROOT, "assets/models.txt")):
        words = line.split("#")[0].split()
        if len(words) >= 2:
            pattern, kind = words[0], words[1]
            (prefixes if pattern.endswith("*") else exact).setdefault(pattern.rstrip("*"), kind)
    return exact, prefixes


def lookup(maps, face):
    exact, prefixes = maps
    base = face.split(".")[0]
    for want in (face, base):
        if want in exact:
            return exact[want]
    hits = [p for p in prefixes if base.startswith(p)]
    return prefixes[max(hits, key=len)] if hits else "card"


def is_wall(face):
    """terrain.rs `is_wall`."""
    base = face.split(".")[0]
    return "wall" in base and not any(w in base for w in ["fire", "lightning", "bul", "speedb", "cloud", "door"])


def pictures():
    """face name -> picture path, for the default (`base`) set."""
    out = {}
    for dirpath, _, files in os.walk(ARCH):
        for fn in files:
            m = re.fullmatch(r"(.+)\.base\.(\w+)\.png", fn)
            if m:
                out[f"{m[1]}.{m[2]}"] = os.path.join(dirpath, fn)
    return out


def map_faces():
    """Picture and animation names that maps set on their objects (`face` / `animation`)."""
    out = set()
    for dirpath, _, files in os.walk(MAPS):
        for fn in files:
            with open(os.path.join(dirpath, fn), errors="ignore") as f:
                for line in f:
                    if line.startswith(("face ", "animation ")):
                        out.add(line.split()[1].split(".")[0] if len(line.split()) > 1 else "")
    return out


def rule_existing(face, users, maps, size, folder):
    """Pictures that already have a 3D look: models.txt, walls, player bodies."""
    kind = lookup(maps, face)
    if kind != "card":
        return "existing", f"models.txt: {kind}"
    if is_wall(face):
        return "existing", "drawn as wall blocks"
    if any(u["folder"].startswith("player") for u in users):
        return "existing", "player character (3D bodies)"
    return None


def rule_icon_or_generator(face, users, maps, size, folder):
    """Spell icons are skipped; monster generators are modelled from scratch."""
    if face.startswith(("spell_", "skill_", "prayer_")):
        return "skip", "a spell or skill icon"
    # A monster that multiplies (centipede, slime) is a generator too, but a creature.
    if any(u.get("generator") == "1" and u.get("monster") != "1" for u in users):
        return "scratch", "a monster generator (a building or object)"
    return None


def rule_folder_first(face, users, maps, size, folder):
    """Folders whose group is fixed before any object flag is looked at."""
    top = folder.split(os.sep)[0]
    if top in FOLDER_FIRST:
        group, why = FOLDER_FIRST[top]
        return group, f"folder {top}/: {why}"
    return None


def rule_flags(face, users, maps, size, folder):
    """Creatures (alive or monster) are rigged; floors are flat."""
    if any(u.get("monster") == "1" or u.get("alive") == "1" for u in users):
        return "rigged", "alive: a creature"
    if any(u.get("is_floor") == "1" for u in users):
        return "flat", "is_floor"
    return None


def rule_type(face, users, maps, size, folder):
    """The group of the commonest object type that BY_TYPE lists."""
    types = Counter(int(u.get("type", 0)) for u in users)
    for group, wanted, why in BY_TYPE:
        hit = next((t for t, _ in types.most_common() if t in wanted), None)
        if hit is not None:
            return group, f"type {NAME[hit]}: {why}"
    return None


def rule_fallback(face, users, maps, size, folder):
    """Invisible objects, big pictures, then the picture's folder."""
    if users and all(u.get("invisible") == "1" for u in users):
        return "skip", "invisible"
    if size[0] > 32 or size[1] > 32:
        return "scratch", f"big picture ({size[0]}x{size[1]})"
    top = (users[0]["folder"] if users else folder).split(os.sep)[0]
    for group, folders in BY_FOLDER.items():
        if top in folders:
            return group, f"folder {top}/" + ("" if users else " (no object uses it)")
    return None


RULES = (rule_existing, rule_icon_or_generator, rule_folder_first, rule_flags, rule_type, rule_fallback)


def classify(face, users, maps, size, folder):
    """(group, reason) for one picture. `users` are the objects that show it; `folder`
    is where the picture is (used when no object shows it)."""
    for rule in RULES:
        found = rule(face, users, maps, size, folder)
        if found:
            return found
    return "review", "no rule fits"


def catalog():
    """One row per picture: (picture, file, size, objects, group, reason)."""
    objects, anims = parse_arch()
    users = defaultdict(list)
    for o in objects:
        shown = set(o["frames"])
        if "face" in o:
            shown.add(o["face"])
        shown.update(anims.get(o.get("animation", ""), []))
        for face in shown:
            users[face].append(o)
    maps = model_map()
    in_maps = map_faces()
    rows = []
    for face, path in sorted(pictures().items()):
        size = Image.open(path).size
        us = users.get(face, [])
        group, why = classify(face, us, maps, size, os.path.relpath(os.path.dirname(path), ARCH))
        # A thing to model that no object or map shows never appears in the game.
        if group == "scratch" and not us and face.split(".")[0] not in in_maps:
            group, why = "skip", "unused: no object or map shows it"
        names = ",".join(sorted({u["name"] for u in us})[:3]) if us else ""
        rows.append((face, os.path.relpath(path, ARCH), f"{size[0]}x{size[1]}", names, group, why))
    return rows


def main(samples):
    rows = catalog()
    with open(os.path.join(HERE, "catalog.tsv"), "w") as f:
        f.write("picture\tfile\tsize\tobjects\tgroup\treason\n")
        f.writelines("\t".join(r) + "\n" for r in rows)
    counts = Counter(r[4] for r in rows)
    # Frames of one thing (goblin.111, goblin.151, ...) need one model between them.
    things = Counter(g for g, _ in {(r[4], r[0].split(".")[0]) for r in rows})
    reasons = Counter((r[4], r[5].split(" (")[0]) for r in rows)
    with open(os.path.join(HERE, "CATALOG.md"), "w") as f:
        f.write("# Picture catalog by 3D technique\n\nMade by `catalog.py` from the `base` picture set "
                f"({len(rows)} pictures). Every picture with its group and reason: `catalog.tsv`. "
                "Sample sheets: `shots/catalog-<group>.jpg`. Things: pictures with the same name before the frame "
                "number (one model each).\n\n| Group | Pictures | Things |\n| --- | ---: | ---: |\n")
        f.writelines(f"| {g} | {n} | {things[g]} |\n" for g, n in counts.most_common())
        f.write("\n## Why (top reasons per group)\n\n| Group | Reason | Pictures |\n| --- | --- | ---: |\n")
        f.writelines(f"| {g} | {r} | {n} |\n" for (g, r), n in sorted(reasons.items(), key=lambda x: (x[0][0], -x[1])) if n >= 10)
    print(dict(counts.most_common()))
    if samples:
        sheets(rows)


def sheets(rows):
    """One sheet of up to 120 random pictures per group, each labelled with its name."""
    by_group = defaultdict(list)
    for r in rows:
        by_group[r[4]].append(r)
    for group, rs in by_group.items():
        pick = sorted(random.Random(0).sample(rs, min(120, len(rs))))
        files = [os.path.join(ARCH, r[1]) for r in pick]
        out = os.path.join(SHOTS, f"catalog-{group}.jpg")
        cmd = ["magick", "montage", "-font", "/System/Library/Fonts/Supplemental/Arial.ttf", "-pointsize", "10",
               "-background", "#d8d8d8", "-label", "%t", *files, "-filter", "point", "-resize", "96x96",
               "-tile", "12x", "-geometry", "96x96+4+4", "-title", f"{group}: {len(rs)} pictures (120 random shown)",
               "-quality", "85", out]
        subprocess.run(cmd, check=True)


def test():
    """The pilot's verdicts (RESULTS.md) still come out of the rules. The pilot's auto items,
    scratch items and creatures now have models (models.txt), so they count as existing."""
    want = {"sword_1.111": "existing", "ring.111": "existing", "ruby.111": "existing", "goblin_head.111": "existing",
            "baby_drag.111": "existing", "book_clasp.111": "existing", "statue.111": "existing",
            "grass.111": "flat", "skull_gen.111": "existing", "centipede.111": "existing", "spell_aggravation.111": "skip",
            "mb_lavawell.111": "skip", "stwall_0.111": "existing"}
    got = {r[0]: r[4] for r in catalog()}
    bad = {f: (got.get(f), g) for f, g in want.items() if got.get(f) != g}
    assert not bad, f"(got, want): {bad}"
    print("ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["test"]:
        test()
    else:
        main(sys.argv[1:] == ["samples"])
