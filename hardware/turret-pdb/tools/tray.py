import os
import sys

from manifold3d import Error, Manifold, OpType

import plates
import stack

SEGMENTS = 96

TRAY = {
    "ceiling": 75.0,
    "tube_d": 12.0,
    "tube_gap": 72.0,
    "tube_bottom": 30.0,
    "bore_d": 12.2,
    "tie_t": 1.5,
    "floor_bottom": 20.0,
    "floor_t": 3.0,
    "half_width": 51.0,
    "length": 155.0,
    "cradles_y": (0.0, 60.0, 143.0),
    "cradle_len": 12.0,
    "cradle_inner": 33.0,
    "tunnels_y": (3.0, 9.0),
    "tunnel": (5.0, 2.0, 25.0),
    "stack2_y": 73.0,
    "boss_d": 7.0,
    "boss_h": 4.0,
    "hole_d": 3.4,
    "board_t": 2.0,
    "esp_above_logic": 17.0,
    "bms": (61.0, 46.0),
    "bms_y": 36.0,
    "bms_board_t": 1.6,
    "mosfet_h": 3.35,
    "foam_t": 1.0,
    "pocket_depth": 1.0,
    "pocket_margin": 0.3,
    "vents_y": (26.0, 36.0, 46.0),
    "vent": (40.0, 4.0),
    "posts": ((-20.0, 8.5), (20.0, 8.5), (-20.0, 63.5), (20.0, 63.5)),
    "post_d": 7.0,
    "deck_bottom": 30.0,
    "deck_t": 3.0,
    "deck_x": 24.0,
    "deck_y": (5.0, 67.0),
    "min_deck_gap": 1.5,
    "pdb_pattern": 30.25,
    "pdb_hole_d": 3.6,
    "pillar_d": 10.0,
    "pdb_bottom": 47.0,
    "pdb_t": 1.6,
    "xt60_h": 19.0,
    "min_tie_gap": 1.0,
    "nut_slot": (5.9, 2.6, 3.0),
    "nut_stop": 3.3,
    "tie_slot": (2.5, 5.0),
    "tie_slots": ((-4.0, 68.0, "y"), (4.0, 68.0, "y"), (-37.0, 32.0, "x"), (-37.0, 40.0, "x"),
                  (37.0, 32.0, "x"), (37.0, 40.0, "x")),
}


def envelope(t=TRAY):
    tube_top = t["tube_bottom"] + t["tube_d"]
    power_board_bottom = t["floor_bottom"] + t["floor_t"] + t["boss_h"]
    logic_bottom = power_board_bottom + t["board_t"] + stack.GAP
    return {
        "tube_top": tube_top,
        "tie_top": tube_top + t["tie_t"],
        "power_board_bottom": power_board_bottom,
        "logic_bottom": logic_bottom,
        "esp_top": logic_bottom + t["board_t"] + t["esp_above_logic"],
        "bms_top": (t["floor_bottom"] + t["floor_t"] - t["pocket_depth"] + t["foam_t"] + t["bms_board_t"]
                    + t["mosfet_h"]),
        "deck_bottom": t["deck_bottom"],
        "pdb_bottom": t["pdb_bottom"],
        "xt60_top": t["pdb_bottom"] + t["pdb_t"] + t["xt60_h"],
    }


def check_envelope(t=TRAY):
    e = envelope(t)
    problems = []
    for key, name in (("esp_top", "ESP"), ("xt60_top", "XT60"), ("tie_top", "zip ties over the tubes")):
        if e[key] > t["ceiling"]:
            problems.append(f"{name} top at z = {e[key]:.1f} is above the {t['ceiling']:.0f} mm ceiling")
    if e["pdb_bottom"] - e["tie_top"] < t["min_tie_gap"]:
        problems.append(f"PDB underside at z = {e['pdb_bottom']:.1f} is within {t['min_tie_gap']} mm of the zip ties "
                        f"at {e['tie_top']:.1f}")
    if e["deck_bottom"] - e["bms_top"] < t["min_deck_gap"]:
        problems.append(f"deck underside at z = {e['deck_bottom']:.1f} is within {t['min_deck_gap']} mm of the BMS "
                        f"MOSFETs at {e['bms_top']:.2f}")
    if stack.W >= t["tube_gap"]:
        problems.append(f"Stack 2 ({stack.W:.1f} mm) does not fit between the tubes ({t['tube_gap']:.1f} mm)")
    if e["logic_bottom"] <= e["tube_top"]:
        problems.append(f"logic board underside at z = {e['logic_bottom']:.1f} is not above the tube tops "
                        f"at {e['tube_top']:.1f}")
    front_cradle_end = t["cradles_y"][0] + t["cradle_len"]
    if t["bms_y"] - t["bms"][1] / 2 < front_cradle_end + 1.0:
        problems.append(f"BMS starts within 1 mm of the front cradle's tie tunnels, "
                        f"which end at y = {front_cradle_end:.1f}")
    if t["bore_d"] - t["tube_d"] < 0.1:
        problems.append(f"cradle bore {t['bore_d']} mm leaves under 0.1 mm around a {t['tube_d']} mm tube")
    return problems


def box(x0, x1, y0, y1, z0, z1):
    return Manifold.cube((x1 - x0, y1 - y0, z1 - z0)).translate((x0, y0, z0))


def zcyl(x, y, z0, z1, d):
    return Manifold.cylinder(z1 - z0, d / 2, d / 2, SEGMENTS).translate((x, y, z0))


def ycyl(x, z, y0, y1, d):
    return Manifold.cylinder(y1 - y0, d / 2, d / 2, SEGMENTS).rotate((-90.0, 0.0, 0.0)).translate((x, y0, z))


def union(parts):
    return Manifold.batch_boolean(list(parts), OpType.Add)


def tube_axis(t):
    return t["tube_gap"] / 2 + t["tube_d"] / 2, t["tube_bottom"] + t["tube_d"] / 2


def floor_top(t):
    return t["floor_bottom"] + t["floor_t"]


def tubes(t=TRAY):
    x, z = tube_axis(t)
    return union(ycyl(s * x, z, -10.0, t["length"] + 10.0, t["tube_d"]) for s in (-1, 1))


def stack2_space(t=TRAY):
    half = stack.W / 2 + 0.5
    y = t["stack2_y"]
    return box(-half, half, y, y + stack.H, envelope(t)["power_board_bottom"], t["ceiling"])


def bms_space(t=TRAY):
    w, l = t["bms"]
    bottom = floor_top(t) - t["pocket_depth"] + t["foam_t"]
    return box(-w / 2, w / 2, t["bms_y"] - l / 2, t["bms_y"] + l / 2, bottom, envelope(t)["bms_top"])


def stack2_holes(t):
    return [(hx - stack.W / 2, hy + t["stack2_y"]) for hx, hy in stack.HOLES]


def vents(t, z0, z1):
    vx, vy = t["vent"]
    return [box(-vx / 2, vx / 2, y - vy / 2, y + vy / 2, z0, z1) for y in t["vents_y"]]


def tray(t=TRAY):
    fb, ft, hw = t["floor_bottom"], floor_top(t), t["half_width"]
    tx, tz = tube_axis(t)
    solid = [box(-hw, hw, 0.0, t["length"], fb, ft)]
    cuts = []
    for y0 in t["cradles_y"]:
        y1 = y0 + t["cradle_len"]
        for s in (-1, 1):
            x0, x1 = sorted((s * t["cradle_inner"], s * hw))
            solid.append(box(x0, x1, y0, y1, fb, tz))
            cuts.append(ycyl(s * tx, tz, y0 - 1.0, y1 + 1.0, t["bore_d"]))
            ty, th, tz0 = t["tunnel"]
            cuts += [box(x0 - 1.0, x1 + 1.0, y0 + c - ty / 2, y0 + c + ty / 2, tz0, tz0 + th) for c in t["tunnels_y"]]
    for x, y in stack2_holes(t):
        solid.append(zcyl(x, y, fb, ft + t["boss_h"], t["boss_d"]))
        cuts.append(zcyl(x, y, fb - 1.0, ft + t["boss_h"] + 1.0, t["hole_d"]))
    for x, y in t["posts"]:
        solid.append(zcyl(x, y, fb, t["deck_bottom"], t["post_d"]))
        cuts.append(zcyl(x, y, fb - 1.0, t["deck_bottom"] + 1.0, t["hole_d"]))
    w, l = (d / 2 + t["pocket_margin"] for d in t["bms"])
    cuts.append(box(-w, w, t["bms_y"] - l, t["bms_y"] + l, ft - t["pocket_depth"], ft + 1.0))
    cuts += vents(t, fb - 1.0, ft + 1.0)
    short, long = t["tie_slot"]
    for x, y, axis in t["tie_slots"]:
        hx, hy = (short / 2, long / 2) if axis == "y" else (long / 2, short / 2)
        cuts.append(box(x - hx, x + hx, y - hy, y + hy, fb - 1.0, ft + 1.0))
    return union(solid) - union(cuts)


def overlaps(a, b):
    return (a ^ b).volume() > 1e-3


def check_parts(t, parts):
    problems = [f"{name}: not one watertight piece" for name, m in parts.items()
                if m.status() != Error.NoError or len(m.decompose()) != 1]
    if "tray" in parts:
        m = parts["tray"]
        if overlaps(m, tubes(t)):
            problems.append("tray: the tubes do not fit the cradles")
        if overlaps(m, stack2_space(t)):
            problems.append("tray: something is in the space Stack 2 takes")
        if overlaps(m, bms_space(t)):
            problems.append("tray: something is in the space the BMS takes")
    if "deck" in parts:
        m = parts["deck"]
        if overlaps(m, parts["tray"] if "tray" in parts else tray(t)):
            problems.append("deck hits the tray")
        if overlaps(m, bms_space(t)):
            problems.append("deck: something is in the space the BMS MOSFETs take")
    return problems


def pillars(t):
    p = t["pdb_pattern"] / 2
    return [(sx * p, t["bms_y"] + sy * p) for sx in (-1, 1) for sy in (-1, 1)]


def deck(t=TRAY):
    db, top, dx = t["deck_bottom"], t["deck_bottom"] + t["deck_t"], t["deck_x"]
    bar = box(-dx, dx, *t["deck_y"], db, top)
    bar -= union([zcyl(x, y, db - 1.0, top + 1.0, t["hole_d"]) for x, y in t["posts"]] + vents(t, db - 1.0, top + 1.0))
    pdb = t["pdb_bottom"]
    w, h, depth = t["nut_slot"]
    cuts = []
    for x, y in pillars(t):
        s = 1.0 if x > 0 else -1.0
        cuts.append(zcyl(x, y, db - 1.0, pdb + 1.0, t["pdb_hole_d"]))
        x0, x1 = sorted((x - s * t["nut_stop"], x + s * (t["pillar_d"] / 2 + 1.0)))
        cuts.append(box(x0, x1, y - w / 2, y + w / 2, pdb - depth - h, pdb - depth))
    return (bar + union(zcyl(x, y, db, pdb, t["pillar_d"]) for x, y in pillars(t))) - union(cuts)


def cradle_test(t=TRAY):
    y0 = t["cradles_y"][0]
    return tray(t) ^ box(30.0, t["half_width"] + 1.0, y0, y0 + t["cradle_len"], t["floor_bottom"] - 1.0, 40.0)


NAMES = ("tray", "deck", "cradle-test")


def main(out, t=TRAY):
    for name in NAMES:
        path = os.path.join(out, f"{name}.stl")
        if os.path.exists(path):
            os.remove(path)
    parts = {"tray": tray(t), "deck": deck(t)}
    problems = check_envelope(t) + check_parts(t, parts)
    if problems:
        sys.exit("\n".join(problems))
    parts["cradle-test"] = cradle_test(t)
    os.makedirs(out, exist_ok=True)
    for name, m in parts.items():
        plates.write(os.path.join(out, f"{name}.stl"), m)
        print(f"{name}: {m.volume() / 1000:.1f} cm3")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "../out/tray")
