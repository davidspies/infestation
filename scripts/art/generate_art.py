#!/usr/bin/env python3
"""Generate all Infestation artwork procedurally (deterministic; fixed seeds).

Run from the repo root:

    python3 scripts/art/generate_art.py                 # atlas + textures
    python3 scripts/art/generate_art.py --review DIR    # also review images (not shipped)

Outputs:
    assets/sprites.png   2048x2048 RGBA atlas (alpha-bled, >= 8px gaps between sprites)
    assets/sprites.json  {"width", "height", "sprites": {name: {x, y, w, h}}}
    assets/textures/{floor,wall_top,wall_front}.jpg  (opaque, so JPEG keeps the wasm small)

Requires pycairo, numpy, scipy and Pillow.
"""

import argparse
import json
from multiprocessing import Pool
from pathlib import Path

import atlas
import board
import creatures
import fx
import icons
import portraits
import textures
import worldmap

REPO = Path(__file__).resolve().parent.parent.parent
ATLAS_SIZE = 2048

# The output contract: sprite names and canvas sizes, in atlas JSON order.
CONTRACT = (
    [("hero1", 256), ("hero2", 256)]
    + [(f"rat_{i}", 256) for i in range(4)]
    + [(f"cyborg_{i}", 256) for i in range(4)]
    + [(n, 256) for n in ("plank", "web", "keg", "hole_base", "hole_swirl", "trigger_plate", "note")]
    + [(n, 128) for n in ("glow", "shadow", "puff", "spark", "star", "ring")]
    + [("slash", 256)]
    + [(n, 128) for n in ("shard", "splinter", "strand", "tuft")]
    + [("confetti", 32)]
    + [
        (f"icon_{n}", 128)
        for n in (
            "undo restart wait map menu gear music sound mute lock check star play close skull sword "
            "rat players export import trophy chevron door"
        ).split()
    ]
    + [("portrait_hero1", 256), ("portrait_hero2", 256)]
    + [(n, 256) for n in ("node_locked", "node_open", "node_done")]
    + [
        (f"prop_{n}", 256)
        for n in (
            "barrel crate kegs bookshelf table cauldron tesla pipes tree bush brazier banner "
            "cheese bones candles rug stairs well flag"
        ).split()
    ]
    + [("title_rat", 512)]
)

BUILDERS = {**creatures.SPRITES, **board.SPRITES, **fx.SPRITES, **icons.SPRITES, **portraits.SPRITES, **worldmap.SPRITES}


def render(item):
    name, size = item
    img = BUILDERS[name]()
    atlas.check_sprite(name, img, size)
    return name, img


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--review", type=Path, help="also write contact_sheet.png, board_mockup.png and atlas_downscaled.png here")
    args = ap.parse_args()

    names = [n for n, _ in CONTRACT]
    if set(BUILDERS) != set(names) or len(names) != len(set(names)):
        raise SystemExit(f"sprite builders and the contract disagree: missing {sorted(set(names) - set(BUILDERS))}, extra {sorted(set(BUILDERS) - set(names))}")

    with Pool() as pool:
        images = dict(pool.map(render, CONTRACT))
        tex = pool.starmap(
            _texture,
            [("floor", textures.floor_texture), ("wall_top", textures.wall_top_texture), ("wall_front", textures.wall_front_texture)],
        )

    sheet, rects = atlas.build_atlas(images, ATLAS_SIZE)
    assets = REPO / "assets"
    sheet.save(assets / "sprites.png", optimize=True)
    (assets / "sprites.json").write_text(json.dumps({"width": ATLAS_SIZE, "height": ATLAS_SIZE, "sprites": rects}, indent=1) + "\n")
    (assets / "textures").mkdir(exist_ok=True)
    for name, img in tex:
        img.convert("RGB").save(assets / "textures" / f"{name}.jpg", quality=90, optimize=True)
    used = sum(r["w"] * r["h"] for r in rects.values())
    print(f"atlas: {len(rects)} sprites, {100 * used / ATLAS_SIZE**2:.1f}% of {ATLAS_SIZE}^2 covered by sprite rects")

    if args.review:
        import review

        args.review.mkdir(parents=True, exist_ok=True)
        review.contact_sheet(images).save(args.review / "contact_sheet.png")
        review.board_mockup(images, dict(tex)).save(args.review / "board_mockup.png")
        review.downscale_check(sheet).save(args.review / "atlas_downscaled.png")
        print(f"review images in {args.review}")


def _texture(name, fn):
    return name, fn()


if __name__ == "__main__":
    main()
