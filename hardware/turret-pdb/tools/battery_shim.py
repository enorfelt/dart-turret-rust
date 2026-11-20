import math
import os
import sys

from manifold3d import CrossSection, Error, OpType

import plates

SEGMENTS = 96

BATTERY_SHIM = {
    "outline": ((38.75, -46.0), (-38.75, -46.0), (-49.1, -59.7), (-38.0, -58.0),
                (-38.0, -90.0), (38.0, -90.0), (38.0, -58.0), (49.1, -59.7)),
    "stacks_y": (-25.0, 25.0),
    "cells_z": (-59.0, -78.0),
    "pocket_d": 19.2,
    "pocket_gap": 0.1,
    "recess": (10.16, 30.48, -68.49),
    "window": ((7.22, -78.0), (10.22, -75.0), (10.22, -58.0), (7.22, -55.0),
               (-7.22, -55.0), (-10.22, -58.0), (-10.22, -75.0), (-7.22, -78.0)),
    "holes": ((-36.0, -50.0), (-10.0, -50.0), (10.0, -50.0), (36.0, -50.0),
              (-34.0, -86.0), (-16.0, -86.0), (16.0, -86.0), (34.0, -86.0)),
    "hole_d": 3.2,
    "wire_slit": (2.0, -68.5),
    "min_wall": 0.6,
    "cover_under_head": 3.0,
    "pilot_depth": 10.0,
    "min_bite": 6.0,
    "thicknesses": (1.0, 1.6, 2.2),
}


def circle(y, z, d):
    return CrossSection.circle(d / 2, SEGMENTS).translate((y, z))


def opening(s, y):
    top, bottom = s["cells_z"]
    d = s["pocket_d"] + 2 * s["pocket_gap"]
    return CrossSection.batch_hull([circle(y, top, d), circle(y, bottom, d)])


def recess(s, y):
    w, h, z = s["recess"]
    return CrossSection.square((w, h), center=True).translate((y, z))


def slit(s, y):
    w, z = s["wire_slit"]
    edge = max(abs(p[0]) for p in s["outline"]) + 1.0
    y0, y1 = sorted((y, math.copysign(edge, y)))
    return CrossSection.square((y1 - y0, w)).translate((y0, z - w / 2))


def profile(s=BATTERY_SHIM):
    cuts = [opening(s, y) for y in s["stacks_y"]] + [slit(s, y) for y in s["stacks_y"]]
    cuts.append(CrossSection([list(s["window"])]))
    cuts += [circle(y, z, s["hole_d"]) for y, z in s["holes"]]
    return CrossSection([list(s["outline"])]) - CrossSection.batch_boolean(cuts, OpType.Add)


def shim(t, s=BATTERY_SHIM):
    return profile(s).extrude(t)


def hole_walls(s=BATTERY_SHIM):
    r = (s["pocket_d"] / 2 + s["pocket_gap"], s["hole_d"] / 2)
    walls = []
    for hy, hz in s["holes"]:
        for y in s["stacks_y"]:
            for z in s["cells_z"]:
                walls.append(((hy - y) ** 2 + (hz - z) ** 2) ** 0.5 - sum(r))
    return min(walls)


def bite(t, screw_len, s=BATTERY_SHIM):
    return screw_len - s["cover_under_head"] - t


def screw_for(t, s=BATTERY_SHIM):
    longest = s["cover_under_head"] + t + s["pilot_depth"]
    return next(n for n in (12, 14, 16, 18, 20) if s["min_bite"] <= bite(t, n, s) and n <= longest)


def check(s=BATTERY_SHIM):
    problems = []
    if hole_walls(s) < s["min_wall"]:
        problems.append(f"screw holes come within {hole_walls(s):.2f} mm of the cell openings")
    for y in s["stacks_y"]:
        if (recess(s, y) - opening(s, y)).area() > 1e-6:
            problems.append("the cover's plate recess sticks out of the shim's cell opening")
    for t in s["thicknesses"]:
        m = shim(t, s)
        if m.status() != Error.NoError or len(m.decompose()) != 1:
            problems.append(f"{t} mm shim: not one watertight piece")
    return problems


def name(t):
    return f"battery-shim-{t:.1f}mm"


def main(out, s=BATTERY_SHIM):
    if os.path.isdir(out):
        for f in os.listdir(out):
            if f.startswith("battery-shim-") and f.endswith(".stl"):
                os.remove(os.path.join(out, f))
    problems = check(s)
    if problems:
        sys.exit("\n".join(problems))
    os.makedirs(out, exist_ok=True)
    for t in s["thicknesses"]:
        m = shim(t, s)
        plates.write(os.path.join(out, f"{name(t)}.stl"), m)
        n = screw_for(t, s)
        print(f"{name(t)}: M3x{n} screws, {bite(t, n, s):.1f} mm into the holder")
    x0, y0, _, x1, y1, _ = shim(1.0, s).bounding_box()
    print(f"outline {x1 - x0:.1f} x {y1 - y0:.1f} mm, {hole_walls(s):.2f} mm between screw holes and cell openings")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "../out/battery-shim")
