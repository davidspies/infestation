"""Atlas packing, validation and alpha bleeding."""

import numpy as np
from PIL import Image
from scipy import ndimage

PAD = 8  # min transparent gap between neighbouring sprite rects
CELL = 128 + PAD  # packing grid pitch: every sprite size is a multiple of 128 (or smaller)
RING = 2  # outermost ring inside each rect that must be fully transparent


def check_sprite(name, img, size):
    if img.size != (size, size):
        raise ValueError(f"{name}: expected {size}x{size}, got {img.size}")
    a = np.asarray(img)[..., 3]
    ring = np.ones_like(a, bool)
    ring[RING:-RING, RING:-RING] = False
    if a[ring].any():
        ys, xs = np.nonzero(a * ring)
        raise ValueError(f"{name}: non-transparent pixels in the {RING}px border ring, e.g. at x={xs[0]} y={ys[0]} (alpha {a[ys[0], xs[0]]})")
    if not a.any():
        raise ValueError(f"{name}: sprite is empty")


def bleed(rgba, region=None):
    """Alpha bleeding: give fully transparent pixels the RGB of the nearest pixel with
    alpha > 0 so straight-alpha bilinear/mipmap filtering never pulls in foreign colours.
    If `region` (bool mask) is given, only pixels inside it are filled."""
    rgba = rgba.copy()
    solid = rgba[..., 3] > 0
    if not solid.any():
        raise ValueError("cannot bleed an empty image")
    idx = ndimage.distance_transform_edt(~solid, return_distances=False, return_indices=True)
    fill = ~solid if region is None else (~solid & region)
    rgba[fill, :3] = rgba[idx[0][fill], idx[1][fill], :3]
    return rgba


def pack(items, atlas_size):
    """Deterministic grid packing. items: list of (name, size) -> {name: (x, y)}.

    The atlas is divided into CELL-pitch slots; a sprite of size s occupies a
    k x k block of slots with k = ceil((s + PAD) / CELL), so neighbours are always
    separated by at least PAD transparent pixels. Larger sprites are placed first,
    each at the first free block in row-major order."""
    n = (atlas_size + PAD) // CELL
    used = np.zeros((n, n), bool)
    order = sorted(range(len(items)), key=lambda i: -items[i][1])
    out = {}
    for i in order:
        name, s = items[i]
        k = -(-(s + PAD) // CELL)
        for r in range(n - k + 1):
            free = [c for c in range(n - k + 1) if not used[r : r + k, c : c + k].any()]
            if free:
                c = free[0]
                used[r : r + k, c : c + k] = True
                out[name] = (c * CELL, r * CELL)
                break
        else:
            raise ValueError(f"atlas full: cannot place {name} ({s}px)")
    return out


def build_atlas(images, atlas_size):
    """images: ordered dict name -> PIL RGBA. Returns (atlas PIL image, rects dict)."""
    items = [(name, img.width) for name, img in images.items()]
    pos = pack(items, atlas_size)
    atlas = np.zeros((atlas_size, atlas_size, 4), np.uint8)
    in_rect = np.zeros((atlas_size, atlas_size), bool)
    rects = {}
    for name, img in images.items():
        x, y = pos[name]
        w, h = img.size
        atlas[y : y + h, x : x + w] = bleed(np.asarray(img))
        in_rect[y : y + h, x : x + w] = True
        rects[name] = {"x": x, "y": y, "w": w, "h": h}
    atlas = bleed(atlas, region=~in_rect)
    return Image.fromarray(atlas, "RGBA"), rects
