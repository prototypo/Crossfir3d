"""Sprite PNG -> GLB via a local image-to-3D model (experiment).

Engines:
  triposr  VAST-AI-Research/TripoSR, weights stabilityai/TripoSR (MIT). Code in upstream/TripoSR.
           torchmcubes is replaced by PyMCubes; texture baked via moderngl, vertex colours if that fails.
  hunyuan  Tencent Hunyuan3D-2mini shape model (Tencent Hunyuan Community Licence: not valid in
           EU/UK/South Korea, see its LICENSE). Code in upstream/Hunyuan3D-2. Shape only (their
           texture painter needs CUDA); vertex colours come from a front planar projection of the input.
--cleanup: Real-ESRGAN RealESRGAN_x4plus_anime_6B (BSD-3) via spandrel, weights in upstream/weights.

  .venv/bin/python ai3d.py --engine triposr [--cleanup] IN.png OUT.glb
"""
import argparse, os, sys, time, types
from pathlib import Path

UP = Path(__file__).resolve().parents[2] / "upstream"
os.environ.setdefault("HF_HOME", str(UP / "hf"))  # keep downloaded weights under upstream/
os.environ.setdefault("HY3DGEN_MODELS", str(UP / "hy3dgen"))

import numpy as np
import torch
import trimesh
from PIL import Image, ImageFilter
from scipy import ndimage

SIZE, RATIO = 512, 0.85
DEV = "mps" if torch.backends.mps.is_available() else "cpu"


def upscale(rgba, cleanup, bg):
    a = rgba.getchannel("A")
    if not cleanup:
        k = SIZE // max(rgba.size)
        return rgba.resize((rgba.width * k, rgba.height * k), Image.NEAREST)
    from spandrel import ModelLoader
    sr = ModelLoader().load_from_file(UP / "weights/RealESRGAN_x4plus_anime_6B.pth").to(DEV).eval()
    rgb = Image.new("RGB", rgba.size, bg)
    rgb.paste(rgba, mask=a)  # transparent pixels -> background so ESRGAN sees no junk colours
    x = torch.from_numpy(np.asarray(rgb, np.float32) / 255).permute(2, 0, 1)[None].to(DEV)
    with torch.no_grad():
        x = sr(sr(x)).clamp(0, 1)  # 4x twice
    rgb = Image.fromarray((x[0].permute(1, 2, 0).cpu().numpy() * 255).round().astype(np.uint8))
    a = a.resize(rgb.size, Image.NEAREST).filter(ImageFilter.GaussianBlur(rgb.width / 48))
    rgb.putalpha(a.point(lambda v: 255 if v >= 128 else 0))
    return rgb


def preprocess(path, cleanup, bg):
    """Returns (centred RGBA SIZExSIZE, same composited on bg)."""
    big = upscale(Image.open(path).convert("RGBA"), cleanup, bg)
    big = big.crop(big.getchannel("A").getbbox())
    s = SIZE * RATIO / max(big.size)
    big = big.resize((max(1, round(big.width * s)), max(1, round(big.height * s))),
                     Image.LANCZOS if cleanup else Image.NEAREST)
    rgba = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    rgba.paste(big, ((SIZE - big.width) // 2, (SIZE - big.height) // 2))
    flat = Image.new("RGB", rgba.size, bg)
    flat.paste(rgba, mask=rgba.getchannel("A"))
    return rgba, flat


def triposr(rgba, flat):
    sys.path.insert(0, str(UP / "TripoSR"))
    import mcubes
    def mc(vol, t):  # torchmcubes.marching_cubes look-alike: verts in (z, y, x) order
        v, f = mcubes.marching_cubes(vol.cpu().numpy(), t)
        return torch.from_numpy(v[:, ::-1].copy()).float(), torch.from_numpy(f.astype(np.int64))
    sys.modules["torchmcubes"] = types.SimpleNamespace(marching_cubes=mc)
    sys.modules.setdefault("rembg", types.ModuleType("rembg"))  # only used for bg removal, which we skip
    from tsr.system import TSR
    from tsr.utils import to_gradio_3d_orientation
    model = TSR.from_pretrained("stabilityai/TripoSR", config_name="config.yaml", weight_name="model.ckpt")
    model.renderer.set_chunk_size(8192)
    model.to(DEV)
    with torch.no_grad():
        codes = model([flat], device=DEV)
    try:
        from tsr.bake_texture import bake_texture
        m = model.extract_mesh(codes, False, resolution=256)[0]
        b = bake_texture(m, model.to("cpu"), codes[0].cpu(), 512)  # their bake queries on CPU
        tex = Image.fromarray((b["colors"] * 255).astype(np.uint8)).transpose(Image.FLIP_TOP_BOTTOM)
        mesh = trimesh.Trimesh(m.vertices[b["vmapping"]], b["indices"], process=False,
                               visual=trimesh.visual.TextureVisuals(uv=b["uvs"], image=tex))
    except Exception as e:
        print("bake failed, using vertex colours:", e)
        mesh = model.to(DEV).extract_mesh(codes, True, resolution=256)[0]
    return to_gradio_3d_orientation(mesh)


def hunyuan(rgba, flat):
    sys.path.insert(0, str(UP / "Hunyuan3D-2"))
    from hy3dgen.shapegen import Hunyuan3DDiTFlowMatchingPipeline
    pipe = Hunyuan3DDiTFlowMatchingPipeline.from_pretrained(
        "tencent/Hunyuan3D-2mini", subfolder="hunyuan3d-dit-v2-mini", variant="fp16",
        device=DEV, dtype=torch.float16 if DEV == "mps" else torch.float32)
    mesh = pipe(image=rgba, num_inference_steps=30, octree_resolution=256, num_chunks=20000,
                generator=torch.manual_seed(0), output_type="trimesh")[0]
    # Front planar projection: mesh x/y bbox -> sprite alpha bbox. Transparent pixels take the
    # nearest opaque colour so edges don't pick up background.
    img = np.asarray(rgba)
    _, idx = ndimage.distance_transform_edt(img[..., 3] < 128, return_indices=True)
    img = img[idx[0], idx[1], :3]
    x0, y0, x1, y1 = rgba.getchannel("A").getbbox()
    v = mesh.vertices
    lo, hi = v.min(0), v.max(0)
    u = (v[:, 0] - lo[0]) / (hi[0] - lo[0])
    w = (hi[1] - v[:, 1]) / (hi[1] - lo[1])
    px = np.clip((x0 + u * (x1 - x0 - 1)).round().astype(int), 0, SIZE - 1)
    py = np.clip((y0 + w * (y1 - y0 - 1)).round().astype(int), 0, SIZE - 1)
    mesh.visual = trimesh.visual.ColorVisuals(mesh, vertex_colors=img[py, px])
    return mesh


ENGINES = {"triposr": (triposr, (128, 128, 128)), "hunyuan": (hunyuan, (255, 255, 255))}

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--engine", choices=ENGINES, required=True)
    ap.add_argument("--cleanup", action="store_true")
    ap.add_argument("inp")
    ap.add_argument("out")
    a = ap.parse_args()
    run, bg = ENGINES[a.engine]
    t = time.time()
    rgba, flat = preprocess(a.inp, a.cleanup, bg)
    flat.save(Path(a.out).with_suffix(".input.png"))
    mesh = run(rgba, flat)
    mesh.export(a.out)
    print(f"{a.out}: {len(mesh.vertices)} verts, {len(mesh.faces)} faces, {time.time() - t:.1f}s")
