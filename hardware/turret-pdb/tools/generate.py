#!/usr/bin/env python3
import importlib
import json
import re
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import parts
import stack
from kicad import S, A, Atom, footprint_node, rotate, sexpr, uid

BOARDS = ("power", "logic")
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
board = NAME = LIB = PROJ = SHEET_UUID = None
STACKUP = {"copper": 0.6, "core": 2.4}


def use(name):
    global board, NAME, LIB, PROJ, SHEET_UUID
    board = importlib.import_module(name)
    NAME = LIB = f"turret-{name}"
    PROJ = os.path.join(ROOT, name)
    SHEET_UUID = uid("sheet", NAME)
    os.makedirs(PROJ, exist_ok=True)
    return board


def fp_of(ref):
    return board.FP[board.PARTS[ref][0]]


def placed(ref):
    x, y, r = board.PARTS[ref][3]
    return x + stack.ORIGIN[0], y + stack.ORIGIN[1], r


def pad_xy(ref, number, index=0):
    fp = fp_of(ref)
    x, y, r = board.PARTS[ref][3]
    p = fp.pad(number)[index]
    dx, dy = rotate(p.x, p.y, r)
    return x + dx, y + dy


def nets_of(ref):
    return {pad: net or f"unconnected-({ref}-{name}-Pad{pad})" for pad, (name, net) in board.PARTS[ref][2].items()}


def all_nets():
    nets = set()
    for ref in board.PARTS:
        nets |= set(nets_of(ref).values())
    return sorted(nets)



def write_footprints():
    d = os.path.join(PROJ, f"{LIB}.pretty")
    os.makedirs(d, exist_ok=True)
    for old in os.listdir(d):
        if old.endswith(".kicad_mod"):
            os.remove(os.path.join(d, old))
    for fp in board.FP.values():
        with open(os.path.join(d, fp.name + ".kicad_mod"), "w", newline="\n") as f:
            f.write(sexpr(footprint_node(fp, LIB)) + "\n")



SYMBOL_NAMES = {
    "esp": "Freenove_ESP32-S3_WROOM", "shifter": "Adafruit_757", "mp1584": "MP1584_Module",
    "h2": "Conn_01x02", "h3": "Conn_01x03", "h3_esc": "Conn_ESC", "h4": "Conn_01x04",
    "wire2_l": "WirePads_01x02", "wire2_r": "WirePads_01x02", "jack": "BarrelJack", "diode": "D_Schottky", "res": "R",
    "cdisc": "C", "cp6": "C_Polarized", "cp10": "C_Polarized", "cp10l": "C_Polarized", "hole": "MountingHole",
    "link": "WirePads_01x05",
}


def symbol_pins(key):
    if key == "hole":
        return []
    ref = next(r for r, p in board.PARTS.items() if p[0] == key)
    pins = []
    seen = set()
    for p in board.FP[key].pads:
        if p.np or p.number in seen:
            continue
        seen.add(p.number)
        name = board.PARTS[ref][2].get(p.number, (p.number, None))[0]
        if key.startswith("h"):
            name = f"Pin_{p.number}"
        pins.append((p.number, name))
    return pins


def symbol_box(npins):
    rows = max(1, (npins + 1) // 2)
    return 2.54 * rows + 2.54


def symbol_layout(key):
    pins = symbol_pins(key)
    half = (len(pins) + 1) // 2 if len(pins) > 2 else 1
    left, right = pins[:half], pins[half:]
    rows = max(len(left), len(right), 1)
    width = 25.4 if key in ("esp", "shifter") else 12.7
    height = rows * 2.54 + 2.54
    top = (rows - 1) * 2.54 / 2
    out = []
    for i, (n, name) in enumerate(left):
        out.append((n, name, -width / 2 - 2.54, round((top - i * 2.54) / 1.27) * 1.27, 0))
    for i, (n, name) in enumerate(right):
        out.append((n, name, width / 2 + 2.54, round((top - i * 2.54) / 1.27) * 1.27, 180))
    return out, width, height


def font():
    return S("effects", S("font", S("size", 1.27, 1.27)))


def hidden_font():
    return S("effects", S("font", S("size", 1.27, 1.27)), S("hide", A("yes")))


def symbol_def(key, lib_prefix=""):
    name = SYMBOL_NAMES[key]
    pins, width, height = symbol_layout(key)
    fp = board.FP[key]
    refchar = {"esp": "U", "shifter": "U", "mp1584": "U", "diode": "D", "res": "R", "cdisc": "C",
               "cp6": "C", "cp10": "C", "cp10l": "C", "hole": "H"}.get(key, "J")
    node = S("symbol", lib_prefix + name,
             S("pin_names", S("offset", 1.016)),
             S("exclude_from_sim", A("no")), S("in_bom", A("yes")), S("on_board", A("yes")),
             S("property", "Reference", refchar, S("at", 0, height / 2 + 1.27, 0), font()),
             S("property", "Value", name, S("at", 0, -height / 2 - 1.27, 0), font()),
             S("property", "Footprint", f"{LIB}:{fp.name}", S("at", 0, 0, 0), hidden_font()),
             S("property", "Datasheet", "", S("at", 0, 0, 0), hidden_font()),
             S("property", "Description", fp.descr, S("at", 0, 0, 0), hidden_font()))
    body = S("symbol", f"{name}_0_1",
             S("rectangle", S("start", -width / 2, height / 2), S("end", width / 2, -height / 2),
               S("stroke", S("width", 0.254), S("type", A("default"))), S("fill", S("type", A("background")))))
    node.append(body)
    unit = S("symbol", f"{name}_1_1")
    for n, pname, x, y, ang in pins:
        unit.append(S("pin", A("passive"), A("line"), S("at", x, y, ang), S("length", 2.54),
                      S("name", pname, font()), S("number", n, font())))
    node.append(unit)
    node.append(S("embedded_fonts", A("no")))
    return node


def write_symbols():
    lib = S("kicad_symbol_lib", S("version", 20241209), S("generator", "turret-pdb-gen"),
            S("generator_version", "10.0"))
    done = set()
    for key in board.FP:
        if SYMBOL_NAMES[key] in done:
            continue
        done.add(SYMBOL_NAMES[key])
        lib.append(symbol_def(key))
    with open(os.path.join(PROJ, f"{LIB}.kicad_sym"), "w", newline="\n") as f:
        f.write(sexpr(lib) + "\n")



SCH_COLUMNS = [
    ["J1", "J2", "D1", "D2", "C2", "R1", "R2", "C1"],
    ["U3", "U4", "U5", "J11", "J10", "H1", "H2", "H3", "H4"],
    ["U1", "U2"],
    ["J3", "J4", "C3", "J5", "J6", "J7", "J8", "J9", "R3", "R4", "J12"],
]


def snap(v):
    return round(v / 1.27) * 1.27


def schematic_positions():
    pos = {}
    x = 40.64
    for col in [c for c in ([r for r in col if r in board.PARTS] for col in SCH_COLUMNS) if c]:
        y = 30.48
        wmax = 0
        for ref in col:
            key = board.PARTS[ref][0]
            _, width, height = symbol_layout(key)
            y += height / 2
            pos[ref] = (snap(x), snap(y))
            y += height / 2 + 12.7
            wmax = max(wmax, width)
        x += wmax + 60.96
    return pos


def write_schematic():
    sch = S("kicad_sch", S("version", 20250114), S("generator", "turret-pdb-gen"),
            S("generator_version", "10.0"), S("uuid", SHEET_UUID), S("paper", "A2"),
            S("title_block", S("title", "Turret controller board"), S("rev", "A"),
              S("comment", 1, "Copper-tape prototype, generated by tools/generate.py")))
    libs = S("lib_symbols")
    done = set()
    for key in board.FP:
        if SYMBOL_NAMES[key] not in done:
            done.add(SYMBOL_NAMES[key])
            libs.append(symbol_def(key, LIB + ":"))
    sch.append(libs)
    pos = schematic_positions()
    labels = []
    for ref, (key, value, pins, _) in board.PARTS.items():
        sx, sy = pos[ref]
        layout, width, height = symbol_layout(key)
        fp = board.FP[key]
        su = uid("sym", ref)
        node = S("symbol", S("lib_id", f"{LIB}:{SYMBOL_NAMES[key]}"), S("at", sx, sy, 0), S("unit", 1),
                 S("exclude_from_sim", A("no")), S("in_bom", A("yes")),
                 S("on_board", A("yes")), S("dnp", A("no")), S("uuid", su),
                 S("property", "Reference", ref, S("at", sx, sy - height / 2 - 1.27, 0), font()),
                 S("property", "Value", value, S("at", sx, sy + height / 2 + 1.27, 0), font()),
                 S("property", "Footprint", f"{LIB}:{fp.name}", S("at", sx, sy, 0), hidden_font()),
                 S("property", "Datasheet", "", S("at", sx, sy, 0), hidden_font()),
                 S("property", "Description", fp.descr, S("at", sx, sy, 0), hidden_font()))
        for n, _, px, py, ang in layout:
            node.append(S("pin", n, S("uuid", uid("pin", ref, n))))
            ax, ay = snap(sx + px), snap(sy - py)
            net = pins.get(n, (None, None))[1]
            if net:
                lab = S("global_label", net, S("shape", A("passive")), S("at", ax, ay, 180 if ang == 0 else 0),
                        S("fields_autoplaced", A("yes")),
                        S("effects", S("font", S("size", 1.27, 1.27)),
                          S("justify", A("right" if ang == 0 else "left"))),
                        S("uuid", uid("label", ref, n)),
                        S("property", "Intersheetrefs", "${INTERSHEET_REFS}", S("at", ax, ay, 0),
                          S("effects", S("font", S("size", 1.27, 1.27)), S("hide", A("yes")))))
            else:
                lab = S("no_connect", S("at", ax, ay), S("uuid", uid("nc", ref, n)))
            labels.append(lab)
        node.append(S("instances", S("project", NAME, S("path", "/" + SHEET_UUID,
                                                         S("reference", ref), S("unit", 1)))))
        sch.append(node)
    sch += labels
    sch.append(S("sheet_instances", S("path", "/", S("page", "1"))))
    sch.append(S("embedded_fonts", A("no")))
    with open(os.path.join(PROJ, f"{NAME}.kicad_sch"), "w", newline="\n") as f:
        f.write(sexpr(sch) + "\n")



LAYERS = [(0, "F.Cu", "signal"), (2, "B.Cu", "signal"), (9, "F.Adhes", "user", "F.Adhesive"),
          (11, "B.Adhes", "user", "B.Adhesive"), (13, "F.Paste", "user"), (15, "B.Paste", "user"),
          (5, "F.SilkS", "user", "F.Silkscreen"), (7, "B.SilkS", "user", "B.Silkscreen"),
          (1, "F.Mask", "user"), (3, "B.Mask", "user"), (17, "Dwgs.User", "user", "User.Drawings"),
          (19, "Cmts.User", "user", "User.Comments"), (21, "Eco1.User", "user", "User.Eco1"),
          (23, "Eco2.User", "user", "User.Eco2"), (25, "Edge.Cuts", "user"), (27, "Margin", "user"),
          (31, "F.CrtYd", "user", "F.Courtyard"), (29, "B.CrtYd", "user", "B.Courtyard"),
          (35, "F.Fab", "user"), (33, "B.Fab", "user")]


def edge(kind, *pts):
    ox, oy = stack.ORIGIN
    stroke = S("stroke", S("width", 0.1), S("type", A("default")))
    u = S("uuid", uid("edge", kind, *pts))
    if kind == "line":
        x1, y1, x2, y2 = pts
        return S("gr_line", S("start", x1 + ox, y1 + oy), S("end", x2 + ox, y2 + oy), stroke, S("layer", "Edge.Cuts"), u)
    sx, sy, mx, my, ex, ey = pts
    return S("gr_arc", S("start", sx + ox, sy + oy), S("mid", mx + ox, my + oy), S("end", ex + ox, ey + oy),
             stroke, S("layer", "Edge.Cuts"), u)


def outline():
    w, h, r = stack.W, stack.H, stack.CORNER
    k = r * (1 - 0.70710678)
    return [edge("line", r, 0, w - r, 0), edge("line", w, r, w, h - r),
            edge("line", w - r, h, r, h), edge("line", 0, h - r, 0, r),
            edge("arc", w - r, 0, w - k, k, w, r), edge("arc", w, h - r, w - k, h - k, w - r, h),
            edge("arc", r, h, k, h - k, 0, h - r), edge("arc", 0, r, k, k, r, 0)]


def stackup():
    c, d = STACKUP["copper"], STACKUP["core"]
    return S("stackup",
             S("layer", "F.Cu", S("type", "copper"), S("thickness", c)),
             S("layer", "dielectric 1", S("type", "core"), S("thickness", d), S("material", "PETG"),
               S("epsilon_r", 3), S("loss_tangent", 0.02)),
             S("layer", "B.Cu", S("type", "copper"), S("thickness", c)),
             S("copper_finish", "None"), S("dielectric_constraints", A("no")))


ENTRY = {"n": (0, -1), "s": (0, 1), "e": (1, 0), "w": (-1, 0)}


def resolve(pt, width=2.0):
    if not isinstance(pt, str):
        return pt
    ref, pad = pt.split(".", 1)
    pad, _, side = pad.partition("@")
    x, y = pad_xy(ref, pad)
    if side:
        fp = fp_of(ref)
        p = fp.pad(pad)[0]
        r = (board.PARTS[ref][3][2] + p.angle) % 180
        w, h = (p.w, p.h) if r == 0 else (p.h, p.w)
        dx, dy = ENTRY[side]
        x += dx * max(0.0, w / 2 - width / 2)
        y += dy * max(0.0, h / 2 - width / 2)
    return x, y


def write_board():
    ox, oy = stack.ORIGIN
    pcb = S("kicad_pcb", S("version", 20260206), S("generator", "turret-pdb-gen"), S("generator_version", "10.0"),
            S("general", S("thickness", STACKUP["core"] + 2 * STACKUP["copper"]), S("legacy_teardrops", A("no"))),
            S("paper", "A4"),
            S("title_block", S("title", "Turret controller board"), S("rev", "A")),
            S("layers", *[[A(str(l[0])), l[1], A(l[2]), *l[3:]] for l in LAYERS]),
            S("setup", stackup(), S("pad_to_mask_clearance", 0),
              S("allow_soldermask_bridges_in_footprints", A("no")),
              S("tenting", S("front", A("yes")), S("back", A("yes")))))
    for ref, (key, value, pins, at) in board.PARTS.items():
        fp = board.FP[key]
        pcb.append(footprint_node(fp, LIB, ref=ref, value=value, at=placed(ref), nets=nets_of(ref),
                                  path=f"/{uid('sym', ref)}", sheetfile=f"{NAME}.kicad_sch"))
    pcb += outline()
    for i, (net, layer, width, pts) in enumerate(board.TRACKS):
        xy = [resolve(p, width) for p in pts]
        for j, (a, b) in enumerate(zip(xy, xy[1:])):
            pcb.append(S("segment", S("start", a[0] + ox, a[1] + oy), S("end", b[0] + ox, b[1] + oy),
                         S("width", width), S("layer", layer), S("net", net), S("uuid", uid("seg", i, j, net))))
    for i, (net, x, y) in enumerate(board.VIAS):
        pcb.append(S("via", S("at", x + ox, y + oy), S("size", 3.0), S("drill", 1.0),
                     S("layers", "F.Cu", "B.Cu"), S("net", net), S("uuid", uid("via", i, net))))
    pcb.append(S("embedded_fonts", A("no")))
    with open(os.path.join(PROJ, f"{NAME}.kicad_pcb"), "w", newline="\n") as f:
        f.write(sexpr(pcb) + "\n")



def netclass(name, width, clearance=1.0, priority=0):
    return {"bus_width": 12, "clearance": clearance, "diff_pair_gap": 0.25, "diff_pair_via_gap": 0.25,
            "diff_pair_width": 0.2, "line_style": 0, "microvia_diameter": 0.5, "microvia_drill": 0.1,
            "name": name, "pcb_color": "rgba(0, 0, 0, 0.000)", "priority": priority,
            "schematic_color": "rgba(0, 0, 0, 0.000)", "track_width": width,
            "via_diameter": 3.0, "via_drill": 1.0, "wire_width": 6}


def write_project():
    classes = [netclass("Default", 2.0, priority=2147483647)]
    patterns = []
    for i, (cls, (width, nets)) in enumerate(board.NETCLASSES.items()):
        classes.append(netclass(cls, width, priority=i))
        patterns += [{"netclass": cls, "pattern": n} for n in nets]
    pro = {
        "board": {"design_settings": {
            "defaults": {"board_outline_line_width": 0.1, "copper_line_width": 0.2},
            "diff_pair_dimensions": [], "drc_exclusions": [],
            "rules": {"min_clearance": 0.5, "min_track_width": 2.0, "min_via_diameter": 3.0,
                      "min_via_annular_width": 0.3, "min_through_hole_diameter": 1.0,
                      "min_hole_clearance": 0.5, "min_annular_width": 0.3, "min_hole_to_hole": 0.5,
                      "min_copper_edge_clearance": 1.0, "min_connection": 0.0,
                      "allow_blind_buried_vias": False, "allow_microvias": False},
            "track_widths": [0.0, 2.0, 3.0, 4.0],
            "via_dimensions": [{"diameter": 0.0, "drill": 0.0}, {"diameter": 3.0, "drill": 1.0}],
            "rule_severities": {"lib_footprint_issues": "ignore", "lib_footprint_mismatch": "ignore",
                                "silk_overlap": "ignore", "silk_over_copper": "ignore",
                                "silk_edge_clearance": "ignore", "courtyards_overlap": "warning",
                                "text_height": "ignore", "text_thickness": "ignore"},
        }},
        "boards": [], "libraries": {"pinned_footprint_libs": [], "pinned_symbol_libs": []},
        "meta": {"filename": f"{NAME}.kicad_pro", "version": 3},
        "net_settings": {"classes": classes, "meta": {"version": 4}, "net_colors": None,
                         "netclass_assignments": None, "netclass_patterns": patterns},
        "pcbnew": {"page_layout_descr_file": ""},
        "schematic": {"meta": {"version": 1}},
        "sheets": [[SHEET_UUID, "Root"]], "text_variables": {},
    }
    with open(os.path.join(PROJ, f"{NAME}.kicad_pro"), "w", newline="\n") as f:
        json.dump(pro, f, indent=2)
        f.write("\n")
    with open(os.path.join(PROJ, f"{NAME}.kicad_dru"), "w", newline="\n") as f:
        f.write("(version 1)\n"
                "(rule \"Pads and vias at 2.54 mm pitch\"\n"
                "\t(constraint clearance (min 0.55mm))\n"
                "\t(condition \"A.Type == 'Pad' || B.Type == 'Pad' || A.Type == 'Via' || B.Type == 'Via'\"))\n"
                "(rule \"Parts are soldered on the back only, front copper reaches them through vias\"\n"
                "\t(layer F.Cu)\n"
                "\t(constraint physical_clearance (min 0.3mm))\n"
                "\t(condition \"A.Type == 'Pad' && (B.Type == 'Track' || B.Type == 'Zone')\"))\n")
    with open(os.path.join(PROJ, "fp-lib-table"), "w", newline="\n") as f:
        f.write(f'(fp_lib_table\n\t(version 7)\n\t(lib (name "{LIB}") (type "KiCad") (uri "${{KIPRJMOD}}/{LIB}.pretty") (options "") (descr "Copper-tape footprints"))\n)\n')
    with open(os.path.join(PROJ, "sym-lib-table"), "w", newline="\n") as f:
        f.write(f'(sym_lib_table\n\t(version 7)\n\t(lib (name "{LIB}") (type "KiCad") (uri "${{KIPRJMOD}}/{LIB}.kicad_sym") (options "") (descr "Turret board symbols"))\n)\n')


FIRMWARE_PINS = {
    "esc_left": "ESC_L", "esc_right": "ESC_R", "pusher": "PUSH_SIG_3V3", "servo_tx": "BUS_TX_3V3",
    "servo_rx": "BUS_RX_3V3", "light_gate": "GATE", "arm_switch": "ARM_SENSE", "battery": "VBAT_SENSE",
}


def members(b, net):
    return {f"{ref}.{pad}" for ref, p in b.PARTS.items() for pad, (_, n) in p[2].items() if n == net}


def nets(b):
    return {n for p in b.PARTS.values() for _, n in p[2].values() if n}


def verify_firmware(logic):
    rs = open(os.path.join(ROOT, "..", "..", "firmware", "turret", "src", "board.rs")).read()
    gpio_of = {pad: name for pad, (name, _) in logic.PARTS["U1"][2].items()}
    for field, net in FIRMWARE_PINS.items():
        m = re.search(rf"{field}: pins\.gpio(\d+)", rs)
        pads = [p for p in members(logic, net) if p.startswith("U1.")]
        assert m and len(pads) == 1, (field, pads)
        assert gpio_of[pads[0][3:]] == m.group(1), f"{field}: board.rs GPIO {m.group(1)}, PCB {pads[0]}"


def verify_power(power):
    assert members(power, "PUSH_6V") == {"J3.1", "J4.2", "C3.1"}, members(power, "PUSH_6V")
    assert members(power, "+6V") == {"U4.VOUT", "J3.2"}, members(power, "+6V")
    assert members(power, "VJACK") == {"J2.1", "D2.2"}, members(power, "VJACK")
    assert members(power, "VBAT") == {"J1.1", "D1.2", "R1.1"}, members(power, "VBAT")
    assert power.FP[power.PARTS["C3"][0]].name.endswith("_Lying"), "C3 must lie flat"
    standing = [r for r, p in power.PARTS.items()
                if power.FP[p[0]].name.startswith("CP_Radial") and not power.FP[p[0]].name.endswith("_Lying")]
    assert standing == ["C2"], f"standing radial capacitors: {standing}"


def esp_local(x, y):
    ex, ey, r = board.PARTS["U1"][3]
    dx, dy = rotate(x, y, r)
    return ex + dx, ey + dy


def verify_logic(logic):
    for ref in ("J5", "J6"):
        assert "2" not in logic.PARTS[ref][2], f"{ref} pin 2 must stay unconnected"
    d = parts.DIM
    a, b = esp_local(-d["esp_end_antenna"], -d["esp_row_spacing"] - d["esp_side"]), esp_local(-1.27, d["esp_side"])
    x1, x2, y1, y2 = min(a[0], b[0]), max(a[0], b[0]), min(a[1], b[1]), max(a[1], b[1])

    def near(x, y, r):
        return x1 - r < x < x2 + r and y1 - r < y < y2 + r

    for net, layer, width, pts in logic.TRACKS:
        xy = [resolve(p, width) for p in pts]
        for (ax, ay), (bx, by) in zip(xy, xy[1:]):
            n = max(1, int(((bx - ax) ** 2 + (by - ay) ** 2) ** 0.5 / 0.5))
            for k in range(n + 1):
                t = k / n
                assert not near(ax + t * (bx - ax), ay + t * (by - ay), width / 2), f"{net} track under the ESP antenna"
    for net, x, y in logic.VIAS:
        assert not near(x, y, 1.5), f"{net} via under the ESP antenna"
    usb = esp_local(19 * parts.P + d["esp_end_usb"], 0)[1]
    assert usb <= 2.0, f"ESP USB end at y = {usb:.2f}, must be within 2 mm of the front edge"


def verify_stack(power, logic):
    for b in (power, logic):
        assert b.PARTS["J12"] == stack.link(), "J12 must be stack.link()"
        assert {r: p for r, p in b.PARTS.items() if r.startswith("H")} == stack.holes(), "holes must be stack.holes()"
        assert b.FP["link"] is stack.LINK_FP
        for net in stack.LINK_NETS:
            assert members(b, net) - {f"J12.{i}" for i in range(1, 6)}, f"{net} reaches nothing but J12"
    shared = nets(power) & nets(logic)
    assert shared == set(stack.LINK_NETS), f"nets on both boards outside the link: {shared - set(stack.LINK_NETS)}"
    labels = [g.pts[2] for g in stack.LINK_FP.graphics if g.kind == "text" and g.layer == "F.SilkS"]
    assert labels == list(stack.LINK_NAMES), labels


def main(names):
    power = use("power")
    verify_power(power)
    logic = use("logic")
    verify_firmware(logic)
    verify_logic(logic)
    verify_stack(power, logic)
    for name in names:
        use(name)
        write_footprints()
        write_symbols()
        write_schematic()
        write_board()
        write_project()


if __name__ == "__main__":
    main(sys.argv[1:] or list(BOARDS))
