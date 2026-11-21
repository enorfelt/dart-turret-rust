import math
import os
import sys

from manifold3d import Error, Manifold, OpType

import plates
import power
import stack
from parts import DIM

SEGMENTS = 96

TRAY = {
    "ceiling": 75.0,
    "tube_d": 12.0,
    "tube_gap": 76.0,
    "tube_bottom": 30.0,
    "bore_d": 12.2,
    "tie_t": 1.5,
    "floor_bottom": 20.0,
    "floor_t": 3.0,
    "half_width": 53.0,
    "length": 155.0,
    "cradles_y": (0.0, 60.0, 143.0),
    "cradle_len": 12.0,
    "cradle_inner": 35.0,
    "tunnels_y": (3.0, 9.0),
    "tunnel": (5.0, 2.0, 23.5),
    "stack2_y": 73.0,
    "boss_d": 7.0,
    "boss_h": 4.0,
    "riser_h": 7.0,
    "buck_under": 7.0,
    "min_floor_gap": 1.0,
    "hole_d": 3.4,
    "board_t": 1.6,
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
    "pdb_parts_h": 4.0,
    "min_tie_gap": 1.0,
    "nut_slot": (5.9, 3.0, 3.0),
    "nut_stop": 3.3,
    "tie_slot": (2.5, 5.0),
    "tie_slots": ((-4.0, 68.0, "y"), (4.0, 68.0, "y"), (-37.0, 32.0, "x"), (-37.0, 40.0, "x"),
                  (37.0, 32.0, "x"), (37.0, 40.0, "x")),
    "sleeve_od": 5.8,
    "sleeve_bore": 3.5,
    "sleeve_chamfer": 0.3,
    "arm_roof": 4.0,
    "arm_gap": 0.3,
    "arm_rear": 16.0,
    "arm_leg_w": 4.0,
    "arm_bar_gap": 1.0,
    "arm_bar_top": 34.0,
    "arm_bar_t": 4.0,
    "cradle_tie_w": 4.8,
    "arm_notch_w": 2.5,
    "switch_x": -15.0,
    "switch_hole_d": 6.3,
    "switch_body": (13.0, 13.0, 18.0),
    "switch_above": 22.0,
    "switch_nut_d": 11.0,
    "min_floor_clear": 5.0,
    "jack_axis": 6.5,
    "plug_d": 12.0,
}


def envelope(t=TRAY):
    tube_top = t["tube_bottom"] + t["tube_d"]
    power_board_bottom = t["floor_bottom"] + t["floor_t"] + t["boss_h"] + t["riser_h"]
    logic_bottom = power_board_bottom + t["board_t"] + stack.GAP
    return {
        "tube_top": tube_top,
        "tie_top": tube_top + t["tie_t"],
        "power_board_bottom": power_board_bottom,
        "bucks_bottom": power_board_bottom - t["buck_under"],
        "logic_bottom": logic_bottom,
        "esp_top": logic_bottom + t["board_t"] + t["esp_above_logic"],
        "bms_top": (t["floor_bottom"] + t["floor_t"] - t["pocket_depth"] + t["foam_t"] + t["bms_board_t"]
                    + t["mosfet_h"]),
        "deck_bottom": t["deck_bottom"],
        "pdb_bottom": t["pdb_bottom"],
        "pdb_top": t["pdb_bottom"] + t["pdb_t"] + t["pdb_parts_h"],
        "switch_top": t["arm_bar_top"] + t["switch_above"],
        "switch_bottom": arm_bar(t)[2] - t["switch_body"][2],
    }


def check_envelope(t=TRAY):
    e = envelope(t)
    problems = []
    for key, name in (("esp_top", "ESP"), ("pdb_top", "PDB parts"), ("tie_top", "zip ties over the tubes"),
                      ("switch_top", "arm switch lever")):
        if e[key] > t["ceiling"]:
            problems.append(f"{name} top at z = {e[key]:.1f} is above the {t['ceiling']:.0f} mm ceiling")
    if e["switch_bottom"] < t["min_floor_clear"]:
        problems.append(f"arm switch body bottom at z = {e['switch_bottom']:.1f} is within "
                        f"{t['min_floor_clear']:.0f} mm of the floor")
    if e["pdb_bottom"] - e["tie_top"] < t["min_tie_gap"]:
        problems.append(f"PDB underside at z = {e['pdb_bottom']:.1f} is within {t['min_tie_gap']} mm of the zip ties "
                        f"at {e['tie_top']:.1f}")
    if e["bucks_bottom"] - floor_top(t) < t["min_floor_gap"]:
        problems.append(f"buck modules under the power board reach z = {e['bucks_bottom']:.1f}, within "
                        f"{t['min_floor_gap']} mm of the floor at {floor_top(t):.1f}")
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
    bore_bottom = tube_axis(t)[1] - t["bore_d"] / 2
    if bore_bottom - tunnel_top(t) < 1.0:
        problems.append(f"tie tunnel roof at z = {tunnel_top(t):.1f} is within 1 mm of the cradle bore "
                        f"at {bore_bottom:.1f}")
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


def tunnel_top(t):
    w, h, z0 = t["tunnel"]
    return z0 + h + w / 2


def tunnel(t, x0, x1, y):
    w, h, z0 = t["tunnel"]
    profile = [(y - w / 2, z0), (y + w / 2, z0), (y - w / 2, z0 + h), (y + w / 2, z0 + h), (y, tunnel_top(t))]
    return Manifold.hull_points([(x, py, pz) for x in (x0, x1) for py, pz in profile])


def floor_top(t):
    return t["floor_bottom"] + t["floor_t"]


def tubes(t=TRAY):
    x, z = tube_axis(t)
    return union(ycyl(s * x, z, -10.0, t["length"] + 10.0, t["tube_d"]) for s in (-1, 1))


def stack2_space(t=TRAY):
    half = stack.W / 2 + 0.5
    y = t["stack2_y"]
    return box(-half, half, y, y + stack.H, envelope(t)["power_board_bottom"], t["ceiling"])


def to_tray(t, bx, by):
    return bx - stack.W / 2, by + t["stack2_y"]


def bucks_space(t=TRAY):
    dx, dy, m = DIM["mp1584_pad_dx"], DIM["mp1584_pad_dy"], DIM["mp1584_margin"]
    top = envelope(t)["power_board_bottom"]
    boxes = []
    for kind, _, _, (px, py, rot) in power.PARTS.values():
        if kind != "mp1584":
            continue
        c, s = round(math.cos(math.radians(rot))), round(math.sin(math.radians(rot)))
        corners = [to_tray(t, px + lx * c + ly * s, py - lx * s + ly * c) for lx in (-m, dx + m) for ly in (-m, dy + m)]
        xs, ys = [x for x, _ in corners], [y for _, y in corners]
        boxes.append(box(min(xs), max(xs), min(ys), max(ys), top - t["buck_under"], top))
    return union(boxes)


def bms_space(t=TRAY):
    w, l = t["bms"]
    bottom = floor_top(t) - t["pocket_depth"] + t["foam_t"]
    return box(-w / 2, w / 2, t["bms_y"] - l / 2, t["bms_y"] + l / 2, bottom, envelope(t)["bms_top"])


def stack2_holes(t):
    return [to_tray(t, hx, hy) for hx, hy in stack.HOLES]


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
            cuts += [tunnel(t, x0 - 1.0, x1 + 1.0, y0 + c) for c in t["tunnels_y"]]
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
        if overlaps(m, bucks_space(t)):
            problems.append("tray: something is in the space the buck modules under the power board take")
    if "riser" in parts:
        z = floor_top(t) + t["boss_h"]
        placed = union(parts["riser"].translate((x, y, z)) for x, y in stack2_holes(t))
        if overlaps(placed, bucks_space(t)):
            problems.append("riser: hits the buck modules under the power board")
    if "arm-bracket" in parts:
        m, base = parts["arm-bracket"], parts["tray"] if "tray" in parts else tray(t)
        if overlaps(m, tubes(t)):
            problems.append("arm bracket: the tubes do not fit its feet")
        if overlaps(m, base):
            problems.append("arm bracket hits the tray")
        if overlaps(m, stack2_space(t)):
            problems.append("arm bracket: something is in the space Stack 2 takes")
        plug = plug_space(t)
        if overlaps(m, plug):
            problems.append("arm bracket: something is in the 12 V plug's path")
        body = switch_space(t)
        if any(overlaps(body, other) for other in (m, base, tubes(t), stack2_space(t), plug)):
            problems.append("arm bracket: the switch body hits the bracket, the tray, the tubes, Stack 2 "
                            "or the 12 V plug's path")
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


def sleeve(t=TRAY, h=stack.GAP):
    r, c = t["sleeve_od"] / 2, t["sleeve_chamfer"]
    body = union([
        Manifold.cylinder(c, r - c, r, SEGMENTS),
        Manifold.cylinder(h - 2 * c, r, r, SEGMENTS).translate((0, 0, c)),
        Manifold.cylinder(c, r, r - c, SEGMENTS).translate((0, 0, h - c)),
    ])
    return body - zcyl(0, 0, -1.0, h + 1.0, t["sleeve_bore"])


def riser(t=TRAY):
    return sleeve(t, t["riser_h"])


def arm_top(t):
    return tube_axis(t)[1] + t["bore_d"] / 2 + t["arm_roof"]


def arm_span(t):
    return t["cradles_y"][-1], t["length"] + t["arm_rear"]


def arm_bar(t):
    top = t["arm_bar_top"]
    return t["length"] + t["arm_bar_gap"], arm_span(t)[1], top - t["arm_bar_t"], top


def switch_centre(t):
    y0, y1, _, _ = arm_bar(t)
    return t["switch_x"], (y0 + y1) / 2


def switch_space(t=TRAY):
    w, l, depth = t["switch_body"]
    x, y = switch_centre(t)
    _, _, under_bar, top = arm_bar(t)
    body = box(x - w / 2, x + w / 2, y - l / 2, y + l / 2, under_bar - depth, under_bar)
    return body + zcyl(x, y, top, top + t["switch_above"], t["switch_nut_d"])


def jack_xz(t):
    bx, by = power.PARTS["J2"][3][:2]
    power_board_bottom = t["floor_bottom"] + t["floor_t"] + t["boss_h"] + t["riser_h"]
    return to_tray(t, bx, by)[0], power_board_bottom + t["board_t"] + t["jack_axis"]


def plug_space(t=TRAY):
    x, z = jack_xz(t)
    r = t["plug_d"] / 2
    return box(x - r, x + r, t["stack2_y"] + stack.H, arm_span(t)[1] + 1.0, z - r, t["ceiling"])


def arm_bracket(t=TRAY):
    tx, tz = tube_axis(t)
    y0, y1 = arm_span(t)
    top, hw, inner = arm_top(t), t["half_width"], t["cradle_inner"]
    by0, by1, bz0, bz1 = arm_bar(t)
    leg = inner - t["arm_leg_w"]
    solid = [box(-inner, inner, by0, by1, bz0, bz1)]
    for s in (-1, 1):
        solid.append(box(*sorted((s * inner, s * hw)), y0, y1, tz + t["arm_gap"], top))
        solid.append(box(*sorted((s * leg, s * inner)), by0, by1, bz0, top))
    notch_end = y0 + t["tunnels_y"][-1] + t["cradle_tie_w"] / 2 + 0.5
    w = t["arm_notch_w"]
    sx, sy = switch_centre(t)
    cuts = [zcyl(sx, sy, bz0 - 1.0, bz1 + 1.0, t["switch_hole_d"])]
    for s in (-1, 1):
        cuts.append(ycyl(s * tx, tz, y0 - 1.0, y1 + 1.0, t["bore_d"]))
        cuts.append(box(*sorted((s * (inner - w + 1.0), s * (inner + 1.0))), y0 - 1.0, notch_end, tz, top + 1.0))
    return union(solid) - union(cuts)


def arm_bracket_print(t=TRAY):
    return arm_bracket(t).rotate((-90.0, 0.0, 0.0))


def stl(m):
    # The tray frame follows KiCad (x right, y rear, z up), which is left-handed; STL is right-handed.
    return m.mirror((1.0, 0.0, 0.0))


NAMES = ("tray", "deck", "cradle-test", "sleeve", "riser", "arm-bracket")


def main(out, t=TRAY):
    for name in NAMES:
        path = os.path.join(out, f"{name}.stl")
        if os.path.exists(path):
            os.remove(path)
    parts = {"tray": tray(t), "deck": deck(t), "arm-bracket": arm_bracket(t), "riser": riser(t)}
    problems = check_envelope(t) + check_parts(t, parts)
    if problems:
        sys.exit("\n".join(problems))
    parts["cradle-test"] = cradle_test(t)
    parts["sleeve"] = sleeve(t)
    parts["arm-bracket"] = arm_bracket_print(t)
    os.makedirs(out, exist_ok=True)
    for name, m in parts.items():
        plates.write(os.path.join(out, f"{name}.stl"), stl(m))
        print(f"{name}: {m.volume() / 1000:.1f} cm3")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "../out/tray")
