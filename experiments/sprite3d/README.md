# Sprite to 3D experiment

Can Crossfire's small 2D pictures be turned into 3D models automatically? This
takes one object from each arch category (`objects.txt`, 26 objects), makes a model
of it in several ways, and takes an in-game screenshot of each: the picture card on
the left, then the models in a row, turned 35° so their depth shows.

Results: `shots/<arch>.jpg`, `shots/contact-sheet.jpg`, and `RESULTS.md`.

## Approaches

| Name | What it does | Runs on |
| --- | --- | --- |
| Extrude | Each opaque pixel becomes a 4-pixel-deep column of voxels (`voxel.py`). | Python, under 1 s |
| Inflated voxels | As extrude, but deeper in the middle (depth grows with the square root of the distance to the outline). | Python, under 1 s |
| TripoSR | Open AI image-to-3D model (MIT), on the sprite scaled up with nearest-neighbour (`ai3d.py`). | Apple GPU (MPS) |
| TripoSR + clean-up | The same on the sprite first upscaled 16× by Real-ESRGAN (anime model). | Apple GPU |
| Hunyuan3D-2mini | Tencent's open image-to-3D model, shape only (its texture painter needs CUDA); colours projected from the front. | Apple GPU |
| Hunyuan3D-2mini + clean-up | The same on the Real-ESRGAN image. | Apple GPU |

Hunyuan3D's licence does not apply in the EU, UK, or South Korea. Hosted engines
(Meshy, Tripo, fal.ai, ...) and TRELLIS.2 (needs CUDA) are not in this run.

## Run it

Once: set up the AI engines (a Python 3.12 venv at `experiments/sprite3d/.venv` with
torch, trimesh, spandrel, PyMCubes, moderngl, xatlas, diffusers, ...; code cloned to
`upstream/TripoSR` and `upstream/Hunyuan3D-2`; about 7 GB of weights under
`upstream/`). Add a DM for the screenshots to the local server:

```sh
echo 'Showcase:devdm:*' >> upstream/install/etc/crossfire/dm_file
```

Then, with `scripts/server.sh run` going:

```sh
python3 -I experiments/sprite3d/run.py convert   # models, about 6 min per object
python3 -I experiments/sprite3d/run.py shoot     # screenshots, about 15 s per object
```

`SPRITE3D_APPROACHES=extrude,inflate` picks approaches. Existing models are kept;
delete one to remake it. The screenshots use the game's compare mode:
`MACCROSSFIRE_COMPARE=file` with lines `picture_name model1.glb model2.glb ...`.
