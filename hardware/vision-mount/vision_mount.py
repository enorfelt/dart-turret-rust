import os
import sys

import numpy as np
from manifold3d import Manifold, Mesh, OpType

SEGMENTS = 96
OpAdd = OpType.Add

MOUNT = {
    "axis_y": 16.9,
    "axis_z": 0.72,
    "bore_d": 16.6,
    "back_x": -127.8,
    "sleeve_front_x": -150.8,
    "riser_y": 20.0,
    "riser_half": 8.0,
    "board_w": 28.0,
    "board_l": 51.0,
    "antenna": 5.6,
    "antenna_gap": 2.3,
    "pcb_t": 1.6,
    "pcb_bottom": 37.0,
    "header_h": 2.5,
    "row_half": 12.7,
    "pitch": 2.54,
    "pins": 20,
    "first_pin": 1.0,
    "slot_w": 1.6,
    "slot_margin": 0.8,
    "dupont": (3.2, 3.4),
    "floor_t": 2.0,
    "front_margin": 3.5,
    "side_clear": 0.25,
    "wall_t": 1.6,
    "window_half": 9.5,
    "rail_half_w": 2.75,
    "skirt_t": 1.6,
    "skirt_bottom": 28.0,
    "ledge_h": 1.0,
    "screw_d": 2.8,
    "screw_depth": 9.0,
    "screw_head_h": 3.0,
    "shield_h": 3.1,
    "shield_gap": 0.3,
    "connector_back": 22.5,
    "bar_l": 8.0,
    "bar_t": 3.0,
    "bar_gap": 0.5,
    "clear_d": 3.4,
    "cam_w": 8.6,
    "cam_side_clear": 0.9,
    "cam_side_t": 2.0,
    "cam_pocket": 8.2,
    "visor_t": 2.0,
    "visor_top": 19.0,
    "lens_slot": (5.8, 7.0, 17.0),
    "lens_flare": 1.0,
    "usb_h": 3.2,
    "cover_t": 1.6,
    "cover_side_t": 1.8,
    "cover_clear": 0.2,
    "cover_gap": 0.5,
    "deck_chamfer": 2.5,
    "ramp": 9.0,
    "hump_gap": 0.3,
    "hump_chamfer": 2.0,
    "hump_tail": 8.0,
    "window": (7.0, 6.5, 17.5),
    "window_flare": 1.0,
    "vents": 4,
    "vent": (1.6, 3.2, 10.5, 18.0),
    "vent_back": 5.0,
    "usb_open": True,
    "mouth_half": 12.5,
    "detents": (-182.0, -164.0, -140.0),
    "detent": (8.0, 0.5, 31.5),
}


def load_stl(path):
    data = open(path, "rb").read()
    n = int.from_bytes(data[80:84], "little")
    dt = np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")])
    tris = np.frombuffer(data[84:84 + n * 50], dtype=dt)["v"].reshape(-1, 3).astype(np.float64)
    verts, idx = np.unique(np.round(tris, 5), axis=0, return_inverse=True)
    m = Manifold(Mesh(vert_properties=verts.astype(np.float32), tri_verts=idx.reshape(-1, 3).astype(np.uint32)))
    if m.status().name != "NoError":
        sys.exit(f"{path}: not a closed mesh ({m.status().name})")
    return m


def write_stl(path, m):
    mesh = m.to_mesh()
    v = np.asarray(mesh.vert_properties)[:, :3]
    t = np.asarray(mesh.tri_verts)
    tri = v[t]
    n = np.cross(tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0])
    n /= np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-12)
    rec = np.zeros(len(t), dtype=[("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")])
    rec["n"] = n
    rec["v"] = tri
    with open(path, "wb") as f:
        f.write(b"vision mount".ljust(80, b" "))
        f.write(len(t).to_bytes(4, "little"))
        f.write(rec.tobytes())


def box(x0, x1, y0, y1, z0, z1):
    return Manifold.cube((x1 - x0, y1 - y0, z1 - z0)).translate((x0, y0, z0))


def xcyl(y, z, x0, x1, d):
    return Manifold.cylinder(x1 - x0, d / 2, d / 2, SEGMENTS).rotate((0.0, 90.0, 0.0)).translate((x0, y, z))


def ycyl(x, z, y0, y1, d):
    return Manifold.cylinder(y1 - y0, d / 2, d / 2, SEGMENTS).rotate((-90.0, 0.0, 0.0)).translate((x, y0, z))


def levels(m=MOUNT):
    pcb_top = m["pcb_bottom"] + m["pcb_t"]
    floor_top = m["pcb_bottom"] - m["header_h"]
    return {
        "pcb_top": pcb_top,
        "floor_top": floor_top,
        "floor_bottom": floor_top - m["floor_t"],
        "bridge": pcb_top + m["shield_h"] + m["shield_gap"],
    }


def board_x(m=MOUNT):
    rear = m["back_x"] - m["antenna_gap"] - m["antenna"]
    return {"rear": rear, "front": rear - m["board_l"]}


def tray_extent(m=MOUNT):
    inner = m["board_w"] / 2 + m["side_clear"]
    boss = inner + 2 * m["rail_half_w"]
    skirt = boss + m["skirt_t"]
    cover_in = skirt + m["cover_clear"]
    return {
        "inner": inner,
        "wall": inner + m["wall_t"],
        "screw": inner + m["rail_half_w"],
        "boss": boss,
        "skirt": skirt,
        "cover_in": cover_in,
        "cover_out": cover_in + m["cover_side_t"],
    }


def x_front(m=MOUNT):
    return board_x(m)["front"] - m["front_margin"]


def bar_x(m=MOUNT):
    flex = board_x(m)["rear"] - m["connector_back"]
    x1 = flex + m["bar_gap"] + m["bar_l"]
    return flex, flex + m["bar_gap"], x1


def screw_x(m=MOUNT):
    _, b0, b1 = bar_x(m)
    return (b0 + b1) / 2


def pin_xs(m=MOUNT):
    front = board_x(m)["front"]
    return [front + m["first_pin"] + i * m["pitch"] for i in range(m["pins"])]


def base(original, m=MOUNT):
    lv = levels(m)
    zc = m["axis_z"]
    e = tray_extent(m)
    xf = x_front(m) + m["cover_t"]
    xb = m["back_x"]
    fb, ft, pt = lv["floor_bottom"], lv["floor_top"], lv["pcb_top"]
    sx = screw_x(m)

    def sym(x0, x1, y0, y1, a, b):
        return [box(x0, x1, y0, y1, zc + a, zc + b), box(x0, x1, y0, y1, zc - b, zc - a)]

    solid = [
        original,
        box(m["sleeve_front_x"], xb, m["riser_y"], fb + 0.01, zc - m["riser_half"], zc + m["riser_half"])
        - xcyl(m["axis_y"], zc, m["sleeve_front_x"] - 1, xb + 1, m["bore_d"]),
        box(xf, xb, fb, ft, zc - e["skirt"], zc + e["skirt"]),
        box(xb - m["cover_t"], xb, fb, lv["bridge"], zc - e["skirt"], zc + e["skirt"]),
    ]
    solid += sym(xf, xb, fb, pt, e["inner"], e["wall"])
    solid += sym(sx - 6.0, xb, fb, lv["bridge"], e["inner"], e["boss"])
    solid += sym(xf, xb, m["skirt_bottom"], pt, e["boss"], e["skirt"])
    solid += sym(xf, xb, m["skirt_bottom"], m["skirt_bottom"] + m["ledge_h"], e["skirt"], e["cover_out"])
    dl, dh, dy = m["detent"]
    for x in m["detents"]:
        for s in (-1, 1):
            flat = box(x - dl / 2, x + dl / 2, dy - 1.5, dy + 1.5, 0, 0.01)
            tip = box(x - dl / 2, x + dl / 2, dy - 0.3, dy + 0.3, 0, dh)
            ridge = Manifold.batch_hull([flat, tip])
            if s < 0:
                ridge = ridge.mirror((0, 0, 1))
            solid.append(ridge.translate((0, 0, zc + s * e["skirt"])))
    body = Manifold.batch_boolean(solid, OpAdd)

    cuts = [box(xf + 3.0, m["sleeve_front_x"] - 1.5, fb - 1, ft + 1, zc - m["window_half"], zc + m["window_half"])]
    xs = pin_xs(m)
    sw = m["slot_w"]
    dw, dlen = m["dupont"]
    for s in (-1, 1):
        zr = zc + s * m["row_half"]
        cuts.append(box(xs[0] - m["slot_margin"], xs[-1] + m["slot_margin"], fb - 1, ft + 1, zr - sw / 2, zr + sw / 2))
        cuts.append(box(xs[0] - dlen / 2, xs[0] + dlen / 2, fb - 1, ft + 1, zr - dw / 2, zr + dw / 2))
        cuts.append(ycyl(sx, zc + s * e["screw"], lv["bridge"] - m["screw_depth"], lv["bridge"] + 1, m["screw_d"]))
    return body - Manifold.batch_boolean(cuts, OpAdd)


def bridge_x(m=MOUNT):
    _, b0, b1 = bar_x(m)
    visor_back = b0 - m["cam_pocket"]
    return {"visor_front": visor_back - m["visor_t"], "visor_back": visor_back, "bar": (b0, b1)}


def bridge_half(m=MOUNT):
    pocket = m["cam_w"] / 2 + m["cam_side_clear"]
    return pocket, pocket + m["cam_side_t"]


def bridge(m=MOUNT):
    lv = levels(m)
    zc = m["axis_z"]
    e = tray_extent(m)
    bxs = bridge_x(m)
    b0, b1 = bxs["bar"]
    y0 = lv["bridge"]
    top = lv["pcb_top"] + m["visor_top"]
    pocket_half, side_half = bridge_half(m)
    visor_front, visor_back = bxs["visor_front"], bxs["visor_back"]

    parts = [
        box(b0, b1, y0, y0 + m["bar_t"], zc - e["boss"], zc + e["boss"]),
        box(visor_front, visor_back, y0, top, zc - side_half, zc + side_half),
        box(visor_front, b1, y0, top, zc - side_half, zc - pocket_half),
        box(visor_front, b1, y0, top, zc + pocket_half, zc + side_half),
    ]
    body = Manifold.batch_boolean(parts, OpAdd)

    sw, s0, s1 = m["lens_slot"]
    ly0, ly1 = lv["pcb_top"] + s0, lv["pcb_top"] + s1
    f = m["lens_flare"]
    slot = box(visor_front - 1, visor_back + 1, ly0, ly1, zc - sw / 2, zc + sw / 2)
    flare = Manifold.batch_hull([
        box(visor_front + m["visor_t"] * 0.5, visor_front + m["visor_t"] * 0.5 + 0.01, ly0, ly1, zc - sw / 2, zc + sw / 2),
        box(visor_front - 0.01, visor_front, ly0 - f, ly1 + f, zc - sw / 2 - f, zc + sw / 2 + f),
    ])
    cuts = [slot, flare]
    for s in (-1, 1):
        cuts.append(ycyl(screw_x(m), zc + s * e["screw"], y0 - 1, y0 + m["bar_t"] + 1, m["clear_d"]))
    return body - Manifold.batch_boolean(cuts, OpAdd)


def cprism(x0, x1, y0, y1, zc, half, c, front=True):
    a = box(x0 + (c if front else 0.0), x1, y0, y1, zc - half + c, zc + half - c)
    b = box(x0, x1, y0, y1 - c, zc - half, zc + half)
    return Manifold.batch_hull([a, b])


def cover_heights(m=MOUNT):
    lv = levels(m)
    t = m["cover_t"]
    return {
        "front": lv["pcb_top"] + m["usb_h"] + m["cover_gap"] + t,
        "rear": lv["bridge"] + m["bar_t"] + m["screw_head_h"] + m["cover_gap"] + t,
        "hump": lv["pcb_top"] + m["visor_top"] + m["cover_gap"] + t,
    }


def cover_shape(m, inset):
    zc = m["axis_z"]
    e = tray_extent(m)
    h = cover_heights(m)
    bxs = bridge_x(m)
    t = m["cover_t"] if inset else 0.0
    side = (e["cover_out"] - e["cover_in"]) if inset else 0.0
    y0 = 0.0 if inset else m["skirt_bottom"] + m["ledge_h"]
    xf = x_front(m)
    xb = m["back_x"]
    shrink = 1.4 if inset else 0.0
    c = m["deck_chamfer"] - shrink
    hump_front = bxs["visor_front"] - m["hump_gap"] - m["cover_t"]
    hump_back = bxs["bar"][1] + m["hump_gap"] + m["cover_t"]
    _, side_half = bridge_half(m)
    hump_half = side_half + m["hump_gap"] + m["cover_t"]
    w = e["cover_out"] - side
    ramp_end = hump_front + m["ramp"]

    front_deck = cprism(xf + t, hump_front, y0, h["front"] - t, zc, w, c)
    deck = front_deck + Manifold.batch_hull([
        cprism(hump_front - 0.01, hump_front, y0, h["front"] - t, zc, w, c, front=False),
        cprism(ramp_end, xb - t, y0, h["rear"] - t, zc, w, c, front=False),
    ])
    hc = m["hump_chamfer"] - shrink
    hump = Manifold.batch_hull([
        cprism(hump_front + t, hump_back - t, y0, h["hump"] - t, zc, hump_half - t, hc),
        box(hump_back + m["hump_tail"], hump_back + m["hump_tail"] + 0.01, y0, h["rear"] - t - 0.5,
            zc - hump_half + t, zc + hump_half - t),
    ])
    return deck + hump, hump_front


def cover(m=MOUNT):
    lv = levels(m)
    zc = m["axis_z"]
    e = tray_extent(m)
    h = cover_heights(m)
    t = m["cover_t"]
    outer, hump_front = cover_shape(m, False)
    inner, _ = cover_shape(m, True)
    body = outer - inner
    xb = m["back_x"]
    xf = x_front(m)

    ww, w0, w1 = m["window"]
    y0, y1 = lv["pcb_top"] + w0, lv["pcb_top"] + w1
    f = m["window_flare"]
    r = ww / 2
    slot = Manifold.batch_hull([
        xcyl(y0 + r, zc, hump_front - 1, hump_front + t + 1, ww),
        xcyl(y1 - r, zc, hump_front - 1, hump_front + t + 1, ww),
    ])
    flare = Manifold.batch_hull([
        xcyl(y0 + r, zc, hump_front + t * 0.5, hump_front + t * 0.5 + 0.01, ww),
        xcyl(y1 - r, zc, hump_front + t * 0.5, hump_front + t * 0.5 + 0.01, ww),
        xcyl(y0 + r, zc, hump_front - 0.01, hump_front, ww + 2 * f),
        xcyl(y1 - r, zc, hump_front - 0.01, hump_front, ww + 2 * f),
    ])
    cuts = [slot, flare, box(xb - t - 1, xb + 1, 0, lv["bridge"] + 0.3, zc - e["cover_in"], zc + e["cover_in"])]

    vw, vp, vz0, vz1 = m["vent"]
    for i in range(m["vents"]):
        x = xb - m["vent_back"] - i * vp
        for s in (-1, 1):
            z0, z1 = sorted((zc + s * vz0, zc + s * vz1))
            cuts.append(Manifold.batch_hull([
                ycyl(x, z0 + vw / 2, h["rear"] - 5, h["rear"] + 1, vw),
                ycyl(x, z1 - vw / 2, h["rear"] - 5, h["rear"] + 1, vw),
            ]))
    if m["usb_open"]:
        cuts.append(box(xf - 1, xf + t + 1, lv["floor_top"], h["front"] - t, zc - m["mouth_half"], zc + m["mouth_half"]))
    dl, dh, dy = m["detent"]
    for x in m["detents"]:
        for s in (-1, 1):
            z0, z1 = sorted((zc + s * (e["cover_in"] - 0.1), zc + s * (e["cover_in"] + dh + 0.3)))
            cuts.append(box(x - dl / 2 - 0.5, x + dl / 2 + 0.5, dy - 1.2, dy + 1.2, z0, z1))
    return body - Manifold.batch_boolean(cuts, OpAdd)


def board_model(m=MOUNT):
    lv = levels(m)
    bx = board_x(m)
    zc = m["axis_z"]
    hw = m["board_w"] / 2
    pcb = box(bx["front"], bx["rear"], m["pcb_bottom"], lv["pcb_top"], zc - hw, zc + hw)
    module = box(bx["rear"] - 20.0, bx["rear"] + m["antenna"], lv["pcb_top"], lv["pcb_top"] + m["shield_h"], zc - 9, zc + 9)
    headers = [box(bx["front"], bx["rear"], lv["floor_top"], m["pcb_bottom"], zc + s * m["row_half"] - 1.27,
                   zc + s * m["row_half"] + 1.27) for s in (-1, 1)]
    pins = [box(x - 0.32, x + 0.32, lv["floor_top"] - 6.0, lv["pcb_top"] + 1.5, zc + s * m["row_half"] - 0.32,
                zc + s * m["row_half"] + 0.32) for x in pin_xs(m) for s in (-1, 1)]
    flex, b0, _ = bar_x(m)
    connector = box(flex - 6.0, flex, lv["pcb_top"], lv["pcb_top"] + 2.2, zc - 8.5, zc + 8.5)
    usb = [box(bx["front"] - 0.5, bx["front"] + 7.5, lv["pcb_top"], lv["pcb_top"] + 3.2, zc + s * 5.5 - 4.5,
               zc + s * 5.5 + 4.5) for s in (-1, 1)]
    cam_front = b0 - m["cam_pocket"] + 0.8
    cam = box(cam_front, cam_front + 4.5, lv["pcb_top"] + 8.0, lv["pcb_top"] + 16.6, zc - m["cam_w"] / 2, zc + m["cam_w"] / 2)
    flex_strip = box(flex + 0.1, flex + 0.4, lv["pcb_top"] + 1.0, lv["pcb_top"] + 16.6, zc - 3, zc + 3)
    return Manifold.batch_boolean([pcb, module, connector, cam, flex_strip] + headers + pins + usb, OpAdd)


def check(parts, board_m):
    problems = []
    named = dict(parts, board=board_m)
    names = list(named)
    for i, a in enumerate(names):
        for b in names[i + 1:]:
            v = (named[a] ^ named[b]).volume()
            if v > 0.05:
                problems.append(f"{a}/{b} overlap {v:.2f} mm3")
    for name, part in parts.items():
        if part.status().name != "NoError" or part.num_vert() == 0:
            problems.append(f"{name}: bad mesh")
    return problems


def main(src, out, m=MOUNT):
    original = load_stl(src)
    parts = {"barrel-vision-mount": base(original, m), "camera-bridge": bridge(m), "vision-cover": cover(m)}
    board = board_model(m)
    problems = check(parts, board)
    if problems:
        sys.exit("\n".join(problems))
    os.makedirs(out, exist_ok=True)
    for name, part in parts.items():
        write_stl(os.path.join(out, f"{name}.stl"), part)
        print(f"{name}: {part.volume() / 1000:.1f} cm3, genus {part.genus()}")
    write_stl(os.path.join(out, "preview-board.stl"), board)


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(os.path.dirname(here))
    src = sys.argv[1] if len(sys.argv) > 1 else os.path.join(root, "tmp", "Turret_FlyWheel_Canon_IRSpeed.stl")
    main(src, sys.argv[2] if len(sys.argv) > 2 else os.path.join(here, "out"))
