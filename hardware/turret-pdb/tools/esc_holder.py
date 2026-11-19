import os
import sys

from manifold3d import CrossSection, Error, Manifold, OpType

import plates

SEGMENTS = 64

ESC_HOLDER = {
    "holes": ((8.25, -18.7), (27.8, -18.7)),
    "hole_d": 3.4,
    "screw_head": (5.6, 3.0),
    "edge_y": -38.0,
    "wall_y": -15.5,
    "tab_inner_x": (1.05, 35.0),
    "tab_outer_x": (-12.95, 45.0),
    "tab_y": -26.39,
    "tab_chamfer": 2.0,
    "tab_t": 3.0,
    "tab_screws": ((-3.95, -21.5), (40.0, -21.5)),
    "gusset": (-12.95, -9.0, -26.39, 19.0),
    "esc": (35.0, 17.0),
    "esc_h": (4.5, 5.9),
    "esc_inner_y": -25.8,
    "gap": 0.5,
    "floor_t": 1.6,
    "clip": (8.0, 1.0),
    "clip_dx": 9.0,
    "root_fillet": 1.0,
    "root_groove": 0.6,
    "grip": 0.4,
    "max_strain": 0.025,
    "side_gap": 0.3,
    "top_gap": 0.2,
    "tie_tab": (8.5, -32.0),
    "tie_slot": (5.2, 2.0, -39.6),
    "slot_wall": 1.2,
    "stop": (1.0, 1.5, -38.0),
}


def box(x0, x1, y0, y1, z0, z1):
    return Manifold.cube((x1 - x0, y1 - y0, z1 - z0)).translate((x0, y0, z0))


def zcyl(x, y, z0, z1, d):
    return Manifold.cylinder(z1 - z0, d / 2, d / 2, SEGMENTS).translate((x, y, z0))


def xcyl(x0, x1, y, z, r):
    return Manifold.cylinder(x1 - x0, r, r, SEGMENTS).rotate((0.0, 90.0, 0.0)).translate((x0, y, z))


def prism(points, z0, z1):
    return CrossSection([points]).extrude(z1 - z0).translate((0.0, 0.0, z0))


def wedge(x0, x1, yz):
    return Manifold.hull_points([(x, y, z) for x in (x0, x1) for y, z in yz])


def union(parts):
    return Manifold.batch_boolean(list(parts), OpType.Add)


def centre(h=ESC_HOLDER):
    (x0, _), (x1, _) = h["holes"]
    return (x0 + x1) / 2


def half_width(h=ESC_HOLDER):
    c = centre(h)
    left, right = h["tab_inner_x"]
    return min(c - left, right - c) - h["gap"]


def esc_y(h=ESC_HOLDER):
    y1 = h["esc_inner_y"]
    return y1 - h["esc"][1], y1


def outer_y(h=ESC_HOLDER):
    return esc_y(h)[0] - h["side_gap"] - h["clip"][1] - h["root_fillet"]


def hook_z(h=ESC_HOLDER):
    return [h["floor_t"] + height + h["top_gap"] for height in h["esc_h"]]


def clip_strain(h=ESC_HOLDER):
    root = h["floor_t"] - h["root_groove"]
    return [1.5 * h["clip"][1] * h["grip"] / (z - root) ** 2 for z in hook_z(h)]


def overhang(h=ESC_HOLDER):
    return h["edge_y"] - outer_y(h)


def mirrored(c, x0, x1):
    return ((c + x0, c + x1), (c - x1, c - x0))


def clips(h):
    c = centre(h)
    w, t = h["clip"]
    y0, y1 = esc_y(h)
    floor, r, g = h["floor_t"], h["root_fillet"], h["root_groove"]
    sg, reach = h["side_gap"], h["side_gap"] + h["grip"]
    solid, cuts = [], []
    for dx, zb in zip((-h["clip_dx"], h["clip_dx"]), hook_z(h)):
        top = zb + reach + 0.3
        x0, x1 = c + dx - w / 2, c + dx + w / 2
        inner, outer = y1 + sg, y0 - sg
        back = inner + t
        solid.append(box(x0, x1, inner, back, 0.0, top))
        solid.append(wedge(x0, x1, [(inner, zb), (inner - reach, zb), (inner - reach, zb + 0.3), (inner, top)]))
        solid.append(box(x0, x1, back, back + r, floor, floor + r) - xcyl(x0 - 1.0, x1 + 1.0, back + r, floor + r, r))
        cuts.append(xcyl(x0, x1, inner - g, floor, g))
        back = outer - t
        solid.append(box(x0, x1, back, outer, 0.0, top))
        solid.append(wedge(x0, x1, [(outer, zb), (outer + reach, zb), (outer + reach, zb + 0.3), (outer, top)]))
        solid.append(box(x0, x1, back - r, back, floor, floor + r) - xcyl(x0 - 1.0, x1 + 1.0, back - r, floor + r, r))
        cuts.append(xcyl(x0, x1, outer + g, floor, g))
    return solid, cuts


def holder(h=ESC_HOLDER):
    c, half = centre(h), half_width(h)
    length = h["esc"][0]
    y0, y1 = esc_y(h)
    out, floor = outer_y(h), h["floor_t"]
    tab_len, tab_y = h["tie_tab"]
    solid = [box(c - half, c + half, out, h["wall_y"] - h["gap"], 0.0, floor),
             box(c - length / 2, c + length / 2, out, y1, 0.0, floor)]
    solid += [box(x0, x1, out, tab_y, 0.0, floor) for x0, x1 in mirrored(c, length / 2, length / 2 + tab_len)]
    st, sh, sy = h["stop"]
    stop_x0 = length / 2 + h["side_gap"]
    solid += [box(x0, x1, y0, sy, 0.0, floor + sh) for x0, x1 in mirrored(c, stop_x0, stop_x0 + st)]
    clip_solid, clip_cuts = clips(h)
    solid += clip_solid
    cuts = clip_cuts + [zcyl(x, y, -1.0, floor + 1.0, h["hole_d"]) for x, y in h["holes"]]
    sw, sd, slot_y = h["tie_slot"]
    end = length / 2 + tab_len - h["slot_wall"]
    cuts += [box(x0, x1, slot_y - sd / 2, slot_y + sd / 2, -1.0, floor + 1.0) for x0, x1 in mirrored(c, end - sw, end)]
    return union(solid) - union(cuts)


def esc_space(h=ESC_HOLDER):
    length = h["esc"][0]
    c, f = centre(h), h["floor_t"]
    y0, y1 = esc_y(h)
    front, back = h["esc_h"]
    return box(c - length / 2, c, y0, y1, f, f + front) + box(c, c + length / 2, y0, y1, f, f + back)


def screw_heads(h=ESC_HOLDER):
    d, height = h["screw_head"]
    return union(zcyl(x, y, h["floor_t"], h["floor_t"] + height, d) for x, y in h["holes"])


def magazine(h=ESC_HOLDER):
    big = 100.0
    wall, tab_y, ch, t = h["wall_y"], h["tab_y"], h["tab_chamfer"], h["tab_t"]
    (li, ri), (lo, ro) = h["tab_inner_x"], h["tab_outer_x"]
    d, height = h["screw_head"]
    gx0, gx1, gy, gz = h["gusset"]
    parts = [box(-big, big, wall, 0.0, 0.0, big), box(gx0, gx1, gy, 0.0, 0.0, gz),
             prism([(lo, tab_y), (li - ch, tab_y), (li, tab_y + ch), (li, wall), (lo, wall)], 0.0, t),
             prism([(ri, wall), (ri, tab_y + ch), (ri + ch, tab_y), (ro, tab_y), (ro, wall)], 0.0, t)]
    parts += [zcyl(x, y, t, t + height, d) for x, y in h["tab_screws"]]
    return union(parts)


def overlaps(a, b):
    return (a ^ b).volume() > 1e-3


def check(h=ESC_HOLDER, m=None):
    m = holder(h) if m is None else m
    problems = []
    if m.status() != Error.NoError or len(m.decompose()) != 1:
        problems.append("holder: not one watertight piece")
    if overlaps(m, magazine(h)):
        problems.append("holder hits the magazine holder's wall, tabs, tab screws or front gusset")
    esc = esc_space(h)
    if overlaps(esc, m) or overlaps(esc, magazine(h)):
        problems.append("the ESC hits the holder or the magazine holder")
    heads = screw_heads(h)
    if overlaps(heads, m) or overlaps(heads, esc):
        problems.append("the holder's screw heads hit its clips or the ESC")
    for end, strain in zip(("motor", "back"), clip_strain(h)):
        if strain > h["max_strain"]:
            problems.append(f"{end} end clips bend to {strain:.1%} strain going over the ESC, "
                            f"above the {h['max_strain']:.1%} PLA takes")
    return problems


NAMES = ("esc-holder", "esc-holder-left", "esc-holder-right")


def main(out, h=ESC_HOLDER):
    for name in NAMES:
        path = os.path.join(out, f"{name}.stl")
        if os.path.exists(path):
            os.remove(path)
    m = holder(h)
    problems = check(h, m)
    if problems:
        sys.exit("\n".join(problems))
    os.makedirs(out, exist_ok=True)
    plates.write(os.path.join(out, "esc-holder-left.stl"), m)
    plates.write(os.path.join(out, "esc-holder-right.stl"), m.mirror((0.0, 1.0, 0.0)))
    x0, y0, z0, x1, y1, z1 = m.bounding_box()
    front, back = clip_strain(h)
    print(f"esc-holder-left, -right: {x1 - x0:.1f} x {y1 - y0:.1f} x {z1 - z0:.1f} mm, {overhang(h):.1f} mm over the "
          f"side edge, clip strain {front:.1%} at the motor end and {back:.1%} at the back")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "../out/esc-holder")
