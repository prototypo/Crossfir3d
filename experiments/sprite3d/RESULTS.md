# Sprite to 3D: results (2026-10-19)

26 objects, 6 approaches, one in-game screenshot each in `shots/` (all on one
page: `shots/contact-sheet.jpg`). In each shot, left to right: 2D card, Extrude,
Inflated voxels, TripoSR, TripoSR + clean-up, Hunyuan3D-2mini, Hunyuan3D-2mini +
clean-up. Everything ran locally on an M3 Max.

## The split: what converts well, and what does not

Automatic conversion works for **thin or simple, front-on objects**. The picture
already shows the whole shape, so a model only has to add depth:

- swords, knives, bows, wands, keys
- jewels, rings, amulets
- goblin heads (and other severed parts), apples, potions
- shields, the blade trap

Automatic conversion does **not** work for objects that the sprite draws from above
at an angle, or that need a real body:

- **Monsters** (goblin, baby dragon): you get a flat cut-out or a lump; they need
  rigs and animations anyway. Use rigged kits, as for players.
- **Box-shaped things** (book, scroll, chest, altar, barrel): the sprite shows top
  and front at once, so every approach bends or squashes it. Easy to model from
  scratch, or from a small set of shared shapes.
- **Statues**: the game already has a better 3D statue (a 3D character in stone).
- **Large or scenic things** (fishboat, cloak, brazier): mixed; better from scratch.
- **Ashes / piles**: every approach fails; Hunyuan made a long broken shape.

## Approaches

| Approach | Verdict | Time per object |
| --- | --- | --- |
| Extrude | Keeps the pixel look exactly; good fallback for thin items. | < 1 s |
| Inflated voxels | Best cheap option: round things (gems, apples, potions) look round. | < 1 s |
| TripoSR | Colours come out almost black; shapes blobby. Not usable. | 15-160 s |
| TripoSR + clean-up | Shapes a little better; still too dark. | 15-180 s |
| Hunyuan3D-2mini | Flat dark slabs from raw pixel art. Not usable. | ~90 s |
| Hunyuan3D-2mini + clean-up | **Best AI result**: smooth swords, knives, rings, heads. | ~90 s |

Clean-up (Real-ESRGAN upscaling before the AI step) matters more than the engine,
as the options doc said: the same engine goes from useless to good.

## Suggested pipeline

1. Thin and simple items: Hunyuan3D + clean-up, with inflated voxels as the
   fallback (and for a retro look). Add decimation: Hunyuan meshes have 200k-670k
   faces.
2. Monsters, furniture, containers, books, statues: 3D kits or models made from
   scratch, mapped in `assets/models.txt` as today.
3. Before real use: Hunyuan's licence does not apply in the EU, UK, or South Korea;
   check it, or try a hosted engine (Meshy, Tripo) on the same 26 objects.
