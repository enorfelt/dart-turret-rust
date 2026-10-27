#!/usr/bin/env python3
import json
import math
import os
import struct
import sys
from collections import defaultdict
from itertools import combinations

import numpy as np
from manifold3d import CrossSection, FillRule, Manifold, OpType

CORE = 0.6
DEPTH = 0.4
RIDGE = 0.4
RIDGE_WIDTH = 0.6
BACKING = 1.8
BOSS = DEPTH - 0.1
BOSS_INSET = 0.15
POCKET_CLEAR = 0.2
POCKET_DEPTH = RIDGE + 0.15
SIMPLIFY = 0.01
WIRED = 2 * (CORE + DEPTH)
GROOVE = 1.0
GROOVE_DEPTH = 0.6
WELL_RING = 0.4
MIN_WALL = 0.4
REACH = 3.0


def region(polys):
    parts = [CrossSection([[(x, -y) for x, y in ring] for ring in rings], FillRule.EvenOdd) for rings in polys]
    out = CrossSection()
    for p in parts:
        out = out + p
    return out


def union(parts):
    return CrossSection.batch_boolean(parts, OpType.Add) if parts else CrossSection()


def strip(x1, y1, x2, y2):
    end = CrossSection.circle(GROOVE / 2, 24)
    return CrossSection.batch_hull([end.translate((x1, -y1)), end.translate((x2, -y2))])


def mirror(cs):
    return cs.mirror((0.0, 1.0))


def slab(cs, height, z=0.0):
    return Manifold.extrude(cs.simplify(SIMPLIFY), height).translate((0.0, 0.0, z))


def raised(outline, copper):
    return slab(outline, CORE) + slab(copper, DEPTH, CORE)


def ridged_base(outline, copper):
    ring = (copper.offset(RIDGE_WIDTH) - copper) ^ outline
    top = CORE + DEPTH
    return slab(outline, top) - slab(copper, DEPTH + 0.01, CORE) + slab(ring, RIDGE, top)


def companion(outline, copper):
    pocket = mirror(copper.offset(RIDGE_WIDTH + POCKET_CLEAR) ^ outline)
    boss = mirror(copper.offset(-BOSS_INSET) ^ outline)
    floor = BACKING - POCKET_DEPTH
    return (slab(mirror(outline), BACKING) - slab(pocket, POCKET_DEPTH + 0.01, floor)
            + slab(boss, POCKET_DEPTH + BOSS, floor))


def centre(polys):
    x0, y0, x1, y1 = region(polys).bounds()
    return (x0 + x1) / 2, -(y0 + y1) / 2


def wire_cuts(shapes, side):
    cuts, holes, length = defaultdict(list), defaultdict(list), 0.0
    for net, polys in shapes["vias"] + (shapes["pads"] if side == "back" else []):
        cuts[net].append(region(polys).offset(WELL_RING))
        holes[net].append(centre(polys))
    for net, *seg in shapes["tracks"][side]:
        runs = [seg]
        # Tracks stop where they touch a pad's copper, but the wire has to reach its hole.
        for end in (seg[:2], seg[2:]):
            near = min(holes[net], key=lambda c: math.dist(c, end), default=None)
            if near and 0 < math.dist(near, end) < REACH:
                runs.append([*end, *near])
        for run in runs:
            cuts[net].append(strip(*run))
            length += math.dist(run[:2], run[2:])
    return cuts, length


def check_walls(cuts, shapes, side):
    near = defaultdict(list, {net: list(parts) for net, parts in cuts.items()})
    for net, polys in shapes["pads"] + shapes["vias"]:
        near[net].append(region(polys))
    nets = {net: union(parts) for net, parts in near.items()}
    grown = {net: cs.offset(MIN_WALL) for net, cs in nets.items()}
    bad = []
    for a, b in combinations(sorted(nets), 2):
        hit = grown[a] ^ nets[b]
        if hit.area() > 1e-4:
            x0, y0, x1, y1 = hit.bounds()
            bad.append(f"{side}: {a} and {b} under {MIN_WALL} mm apart near ({(x0 + x1) / 2:.1f}, {-(y0 + y1) / 2:.1f})")
    return bad


def wired(outline, front, back):
    top = WIRED - GROOVE_DEPTH
    return (slab(outline, WIRED) - slab(back, GROOVE_DEPTH + 0.01, -0.01)
            - slab(front, GROOVE_DEPTH + 0.01, top)).rotate((180.0, 0.0, 0.0))


def write(path, m):
    mesh = m.to_mesh()
    v = np.asarray(mesh.vert_properties)[:, :3]
    t = np.asarray(mesh.tri_verts)
    v = v - [v[:, 0].min(), v[:, 1].min(), v[:, 2].min()]
    tri = v[t]
    nrm = np.cross(tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0])
    nrm /= np.maximum(np.linalg.norm(nrm, axis=1, keepdims=True), 1e-12)
    rec = np.zeros(len(t), dtype=np.dtype([("n", "<3f4"), ("v", "<9f4"), ("a", "<u2")]))
    rec["n"] = nrm
    rec["v"] = tri.reshape(-1, 9)
    with open(path, "wb") as f:
        f.write(b"turret-pdb".ljust(80, b" "))
        f.write(struct.pack("<I", len(t)))
        f.write(rec.tobytes())
    _, idx = np.unique(rec["v"].reshape(-1, 3), axis=0, return_inverse=True)
    tri = idx.reshape(-1, 3)
    edges = np.sort(np.concatenate([tri[:, [0, 1]], tri[:, [1, 2]], tri[:, [2, 0]]]), axis=1)
    _, counts = np.unique(edges, axis=0, return_counts=True)
    bad = int((counts != 2).sum())
    if bad:
        sys.exit(f"{path}: {bad} non-manifold edges")


def main(shapes_json, out):
    shapes = json.load(open(shapes_json))
    holes = region(shapes["holes"])
    outline = region(shapes["outline"]) - holes
    copper = {side: (region(shapes[side]) - holes) ^ outline for side in ("front", "back")}
    sides = {"front": (outline, copper["front"]), "back": (mirror(outline), mirror(copper["back"]))}
    for name, (o, c) in sides.items():
        parts = {"raised": raised(o, c), "base": ridged_base(o, c), "companion": companion(o, c)}
        for kind, m in parts.items():
            write(os.path.join(out, f"{name}-{kind}.stl"), m)
        print(f"{name}: " + ", ".join(f"{k} {m.volume():.0f} mm3" for k, m in parts.items()))
    cuts, length, bad = {}, {}, []
    for side in ("front", "back"):
        nets, length[side] = wire_cuts(shapes, side)
        bad += check_walls(nets, shapes, side)
        cuts[side] = union([p for parts in nets.values() for p in parts]) ^ outline
    if bad:
        sys.exit("\n".join(bad))
    plate = wired(outline, cuts["front"], cuts["back"])
    write(os.path.join(out, "wired.stl"), plate)
    print(f"wired: {plate.volume():.0f} mm3, wire {length['front']:.0f} mm front + {length['back']:.0f} mm back"
          f" + {len(shapes['vias'])} vias")
    preview = (slab(outline, 2 * CORE) + slab(copper["front"], DEPTH, 2 * CORE)
               + slab(copper["back"], DEPTH, -DEPTH))
    write(os.path.join(out, "preview.stl"), preview)


if __name__ == "__main__":
    main(*sys.argv[1:3])
