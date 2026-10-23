#!/usr/bin/env python3
"""Makes a 3D model for every thing in the catalog's `auto` group (overnight run).

    python3 -I experiments/sprite3d/auto.py

One model per thing (all frames of `sword_1` share one), from its first frame:
- assets/models/experiment/auto/hunyuan/<thing>.glb  Hunyuan3D-2mini + clean-up, ~20k faces
- assets/models/experiment/auto/inflate/<thing>.glb  inflated voxels, the fallback
Both scaled and placed like the picture card (normalize.py, voxel.py). Finished models
are kept, so a stopped run carries on where it was. Log: auto.log next to the models.
"""
import csv
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
ARCH = os.path.join(ROOT, "upstream/crossfire-arch")
OUT = os.path.join(ROOT, "assets/models/experiment/auto")
PY = os.path.join(HERE, ".venv/bin/python")
FACES = "20000"


def things():
    """thing -> picture file (its lowest frame number), for the `auto` group."""
    pick = {}
    with open(os.path.join(HERE, "catalog.tsv")) as f:
        for row in csv.DictReader(f, delimiter="\t"):
            if row["group"] == "auto":
                thing = row["picture"].split(".")[0]
                pick[thing] = min(pick.get(thing, row["file"]), row["file"])
    return dict(sorted(pick.items()))


def main():
    """Builds the inflated models, runs one Hunyuan batch for the rest and scales each result like its picture card."""
    for d in ("hunyuan", "inflate", "raw"):
        os.makedirs(os.path.join(OUT, d), exist_ok=True)
    todo = things()
    log = open(os.path.join(OUT, "auto.log"), "a")
    started = time.time()
    for thing, pic in todo.items():
        out = os.path.join(OUT, "inflate", thing + ".glb")
        if not os.path.exists(out):
            subprocess.run([sys.executable, "-I", f"{HERE}/voxel.py", "inflate", os.path.join(ARCH, pic), out])
    # One batch for the AI, so its models load once.
    jobs = [(t, p) for t, p in todo.items() if not os.path.exists(os.path.join(OUT, "hunyuan", t + ".glb"))]
    print(f"{len(todo)} things, {len(jobs)} still to make", flush=True)
    batch = os.path.join(OUT, "batch.txt")
    with open(batch, "w") as f:
        f.writelines(f"{os.path.join(ARCH, p)} {os.path.join(OUT, 'raw', t + '.glb')}\n" for t, p in jobs if
                     not os.path.exists(os.path.join(OUT, "raw", t + ".glb")))
    ai = subprocess.Popen([PY, f"{HERE}/ai3d.py", "--engine", "hunyuan", "--cleanup", "--faces", FACES, "--batch", batch],
                          stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    done = 0
    for line in ai.stdout:
        log.write(line)
        log.flush()
        if "verts," not in line and "FAILED" not in line:
            continue
        done += 1
        rate = (time.time() - started) / done
        print(f"{done}/{len(jobs)}  {line.split('/')[-1].strip()}  ~{rate * (len(jobs) - done) / 3600:.1f} h left", flush=True)
    ai.wait()
    # Scale each raw model like its picture card.
    for t, p in jobs:
        raw, out = os.path.join(OUT, "raw", t + ".glb"), os.path.join(OUT, "hunyuan", t + ".glb")
        if os.path.exists(raw) and not os.path.exists(out):
            subprocess.run([PY, f"{HERE}/normalize.py", raw, os.path.join(ARCH, p), out])
    made = sum(os.path.exists(os.path.join(OUT, "hunyuan", t + ".glb")) for t in todo)
    print(f"done: {made}/{len(todo)} AI models, {(time.time() - started) / 3600:.1f} h", flush=True)


if __name__ == "__main__":
    main()
