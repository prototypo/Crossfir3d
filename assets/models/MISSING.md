# Missing models

The models here come from the free ("Standard") tiers of the Quaternius packs and the
Kenney Castle Kit. All are CC0 (see `LICENSE.txt` in each folder). `scripts/pick_models.py`
copies them from `upstream/packs/` and shrinks textures to 1024 px. The weapons in `weapons/` are
Quaternius' Medieval Weapons Pack (CC0), converted from its OBJ files with
`scripts/obj2gltf.py OUT_DIR file.obj...`. `dungeon/` holds the horse statue and the
plinth from Quaternius' Modular Dungeon pack (CC0), converted the same way; the horse is
then centred (its node `translation`). `props/` holds chests, keys, chairs and tables from
Quaternius' Fantasy Props MegaKit (CC0). `kaykit/` holds stairs and torches from KayKit
Dungeon Remastered (CC0). `hexagon/` holds mountains, hills, the well and the wheelbarrow
from the KayKit Medieval Hexagon Pack (CC0). `platformer/` holds the ladder from
Quaternius' Platformer Pack (CC0), converted with `scripts/obj2gltf.py`. `auto/` holds
small items made from their pictures by `experiments/sprite3d/auto.py`.

| Missing | What we use now | Where to get the real one |
|---|---|---|
| Blood well | Built from shapes in code (`building well`) | The hexagon well, tinted, if it looks right |
| Fountain | Built from shapes in code (`building fountain`) | No pack has one |
| Statues of people and knights | A half-size stone 3D character on the plinth (`statue` in models.txt) | No free pack found |
| Gargoyle statue | The horse statue | No free pack found |
| Clock | Built from shapes in code (`building clock`) | No free pack found |
| Broom for the sweeper woman (charwoman) | Nothing in the hand | No pack here has one |
| Lever | Built from shapes in code (`building lever`, `lever_pulled`) | No free pack found |
| Signpost | Built from shapes in code (`building signpost`) | No pack has one |
| Finished houses | Assembled from village wall and roof pieces (7 recipes) | — (the pack has pieces only) |
| Head-only body part (to wear under outfits) | The full body, with everything below the neck hidden in code | Universal Base Characters **Pro/Source** |
| Regular (non-hero) body build, teen build | Superhero body, scaled per race | Universal Base Characters **Pro/Source** |
| More outfits (wizard robe, plate armour) | Peasant outfit, tinted per race or class | Modular Character Outfits – Fantasy **Pro/Source** |
| Crossbows, staffs, tools (shovel, saw, pipe) in hand | Nothing in the hand | No pack here has them |
| Other players' weapons | Not shown: the server sends only your own items | — |
| Monster models | Bestiary kit (`monsters/`) for goblins, kobolds, imps, skeletons, gnolls, trolls, ogres, demons; orcs are tinted 3D people; the rest are picture cards | More Quaternius monster packs |
