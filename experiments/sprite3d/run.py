#!/usr/bin/env python3
"""Sprite -> 3D experiment: convert one object per Crossfire category with several
approaches, then take an in-game screenshot of each (picture card + models in a row).

    python3 -I experiments/sprite3d/run.py convert   # make the models
    python3 -I experiments/sprite3d/run.py shoot     # screenshots (local server running)
    python3 -I experiments/sprite3d/run.py [all]     # both

Models go to assets/models/experiment/sprite3d/<approach>/<arch>.glb (not in git),
screenshots to experiments/sprite3d/shots/<arch>.jpg. Existing models are kept; delete
one to remake it. Each screenshot needs a test map (upstream/crossfire-maps/experiment/,
written here) and a DM character to `goto` it (see README.md here).
"""
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
ARCH = os.path.join(ROOT, "upstream/crossfire-arch")
MODELS = os.path.join(ROOT, "assets/models")
OUT = os.path.join(MODELS, "experiment/sprite3d")
SHOTS = os.path.join(HERE, "shots")
MAPS = os.path.join(ROOT, "upstream/crossfire-maps/experiment")
VENV_PY = os.path.join(HERE, ".venv/bin/python")
LOGIN = os.environ.get("SPRITE3D_LOGIN", "sprite3d:sprite3d:Showcase:new")
DM_PASSWORD = os.environ.get("SPRITE3D_DM", "devdm")

# (name, label, command). {src} picture, {out} GLB. AI engines run in the venv.
APPROACHES = [
    ("extrude", "Extrude", [sys.executable, "-I", f"{HERE}/voxel.py", "extrude", "{src}", "{out}"]),
    ("inflate", "Inflated voxels", [sys.executable, "-I", f"{HERE}/voxel.py", "inflate", "{src}", "{out}"]),
    ("triposr", "TripoSR", [VENV_PY, f"{HERE}/ai3d.py", "--engine", "triposr", "{src}", "{out}"]),
    ("triposr_clean", "TripoSR + clean-up", [VENV_PY, f"{HERE}/ai3d.py", "--engine", "triposr", "--cleanup", "{src}", "{out}"]),
    ("hunyuan", "Hunyuan3D-2mini", [VENV_PY, f"{HERE}/ai3d.py", "--engine", "hunyuan", "{src}", "{out}"]),
    ("hunyuan_clean", "Hunyuan3D-2mini + clean-up", [VENV_PY, f"{HERE}/ai3d.py", "--engine", "hunyuan", "--cleanup", "{src}", "{out}"]),
]
ENABLED = os.environ.get("SPRITE3D_APPROACHES", ",".join(a[0] for a in APPROACHES)).split(",")
AI_RAW = ("triposr", "hunyuan")  # AI output needs scaling to the picture (normalize.py)


def objects():
    for line in open(os.path.join(HERE, "objects.txt")):
        words = line.split("#")[0].split()
        if words:
            yield words  # category, arch, picture


def face(picture):
    return os.path.basename(picture).split(".base.")[0]


def approaches():
    return [a for a in APPROACHES if a[0] in ENABLED]


def convert():
    for _, arch, picture in objects():
        src = os.path.join(ARCH, picture)
        for name, _, cmd in approaches():
            out = os.path.join(OUT, name, arch + ".glb")
            if os.path.exists(out):
                continue
            os.makedirs(os.path.dirname(out), exist_ok=True)
            started = time.time()
            raw = out + ".raw.glb" if name.startswith(AI_RAW) else out
            ok = subprocess.run([c.format(src=src, out=raw) for c in cmd]).returncode == 0
            if ok and raw != out:
                ok = subprocess.run([VENV_PY, f"{HERE}/normalize.py", raw, src, out]).returncode == 0
            print(f"{arch:14} {name:14} {'ok' if ok else 'FAILED'} {time.time() - started:6.1f} s", flush=True)


# The player starts at (5, 8) facing north; the object stands 3 squares ahead, left
# of the middle, so the card and the models beside it are centred in view.
MAP = """arch map
name Sprite3D {arch}
width 11
height 11
enter_x 5
enter_y 8
outdoor 1
end
{floor}arch {arch}
x {x}
y 5
{extra}end
"""
EXTRA = {"monster": "stand_still 1\nunaggressive 1\n", "traps": "type 0\ninvisible 0\nspeed 0\n"}


def write_map(category, arch, models):
    """Returns the map path. A new name each time: the server keeps maps it has loaded."""
    floor = "".join(f"arch grass\nx {x}\ny {y}\nend\n" for x in range(11) for y in range(11))
    x = max(0, 5 - models // 2)  # models are 1 square apart
    os.makedirs(MAPS, exist_ok=True)
    for old in os.listdir(MAPS):
        if old.startswith(f"sprite3d_{arch}_"):
            os.remove(os.path.join(MAPS, old))
    name = f"sprite3d_{arch}_{int(time.time())}"
    with open(os.path.join(MAPS, name), "w") as f:
        f.write(MAP.format(arch=arch, floor=floor, x=x, extra=EXTRA.get(category, "")))
    return f"/experiment/{name}"


def shoot():
    os.makedirs(SHOTS, exist_ok=True)
    subprocess.run(["cargo", "build", "--release", "-p", "maccrossfire"], cwd=ROOT, check=True)
    for category, arch, picture in objects():
        files = [f"experiment/sprite3d/{n}/{arch}.glb" for n, _, _ in approaches() if os.path.exists(os.path.join(OUT, n, arch + ".glb"))]
        labels = ["2D card"] + [lbl for n, lbl, _ in approaches() if os.path.exists(os.path.join(OUT, n, arch + ".glb"))]
        map_path = write_map(category, arch, len(files))
        compare = os.path.join(OUT, f"compare_{arch}.txt")
        with open(compare, "w") as f:
            f.write(f"{face(picture)} {' '.join(files)}\n")
        shot = os.path.join(SHOTS, arch + ".png")
        env = dict(
            os.environ,
            MACCROSSFIRE_LOGIN=LOGIN,
            MACCROSSFIRE_COMMANDS=f"dm {DM_PASSWORD};goto {map_path}",
            MACCROSSFIRE_COMPARE=compare,
            MACCROSSFIRE_SCREENSHOT=shot,
            MACCROSSFIRE_SCREENSHOT_AT="10",
            MACCROSSFIRE_HOUR="14",
        )
        subprocess.run([os.path.join(ROOT, "target/release/maccrossfire")], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        ok = os.path.exists(shot)
        if ok:
            label(shot, f"{category}: {arch}", labels)
        print(f"{arch:14} {'ok' if ok else 'NO SCREENSHOT'}", flush=True)


def label(shot, title, labels):
    """Adds a title and the left-to-right order under the screenshot."""
    text = f"{title}\nleft to right:  " + "  |  ".join(labels)
    subprocess.run(["magick", shot, "-resize", "1600x", "(", "-size", "1600x80", "-background", "#202020", "-fill", "white",
                    "-font", "/System/Library/Fonts/Supplemental/Arial.ttf", "-pointsize", "22", "-gravity", "center", f"label:{text}", ")", "-append", "-quality", "85", shot[:-4] + ".jpg"], check=True)
    os.remove(shot)


if __name__ == "__main__":
    step = sys.argv[1] if len(sys.argv) > 1 else "all"
    if step in ("convert", "all"):
        convert()
    if step in ("shoot", "all"):
        shoot()
