import stack

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
    "length": 150.0,
    "cradles_y": (0.0, 55.0, 138.0),
    "cradle_len": 12.0,
    "cradle_inner": 33.0,
    "tunnels_y": (3.0, 9.0),
    "tunnel": (5.0, 2.0, 25.0),
    "stack2_y": 68.0,
    "boss_d": 7.0,
    "boss_h": 4.0,
    "hole_d": 3.4,
    "board_t": 2.0,
    "stack2_height": 38.0,
    "bms": (61.0, 46.0),
    "bms_y": 31.0,
    "bms_board_t": 1.6,
    "mosfet_h": 3.35,
    "foam_t": 1.0,
    "pocket_depth": 1.0,
    "pocket_margin": 0.3,
    "vents_y": (21.0, 31.0, 41.0),
    "vent": (40.0, 4.0),
    "posts": ((-20.0, 3.5), (20.0, 3.5), (-20.0, 58.5), (20.0, 58.5)),
    "post_d": 7.0,
    "deck_bottom": 30.0,
    "deck_t": 3.0,
    "deck_x": 24.0,
    "deck_y": (0.0, 62.0),
    "min_deck_gap": 1.5,
    "pdb_pattern": 30.25,
    "pdb_hole_d": 3.6,
    "pillar_d": 8.0,
    "pdb_bottom": 47.0,
    "pdb_t": 1.6,
    "xt60_h": 19.0,
    "min_tie_gap": 1.0,
    "nut_slot": (5.7, 2.6, 3.0),
    "tie_slot": (2.5, 5.0),
    "tie_slots": ((-4.0, 63.0, "y"), (4.0, 63.0, "y"), (-37.0, 27.0, "x"), (-37.0, 35.0, "x"),
                  (37.0, 27.0, "x"), (37.0, 35.0, "x")),
}


def envelope(t=TRAY):
    tube_top = t["tube_bottom"] + t["tube_d"]
    power_board_bottom = t["floor_bottom"] + t["floor_t"] + t["boss_h"]
    return {
        "tube_top": tube_top,
        "tie_top": tube_top + t["tie_t"],
        "power_board_bottom": power_board_bottom,
        "logic_bottom": power_board_bottom + t["board_t"] + stack.GAP,
        "esp_top": power_board_bottom + t["stack2_height"],
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
    if t["bore_d"] - t["tube_d"] < 0.1:
        problems.append(f"cradle bore {t['bore_d']} mm leaves under 0.1 mm around a {t['tube_d']} mm tube")
    return problems
