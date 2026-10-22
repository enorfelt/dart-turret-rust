from kicad import Footprint, Graphic, Pad

P = 2.54
MODELS = "${KICAD10_3DMODEL_DIR}"

DIM = {
    "esp_row_spacing": 25.4,
    "esp_end_antenna": 7.5,
    "esp_end_usb": 1.3,
    "esp_side": 1.3,
    "shifter_row_spacing": 10.16,
    "mp1584_pad_dx": 19.0,
    "mp1584_pad_dy": 14.0,
    "mp1584_margin": 1.5,
    "pin_drill": 1.2,
    "pin_pad": (1.9, 4.0),
    "pin_pad_single": (1.9, 3.0),
    "lead_drill": 1.2,
    "lead_pad": 2.6,
    "wire_drill": 1.6,
    "wire_pad": 3.8,
    "wire_pitch": 6.0,
    "power_lead_drill": 1.7,
    "power_lead_pad": 3.8,
    "mount_drill": 3.2,
    "c3_length": 20.0,
}

PIN = DIM["pin_pad"]
PD = DIM["pin_drill"]

ESP_PINS = {
    "L": ["3V3", "EN", "4", "5", "6", "7", "15", "16", "17", "18",
          "8", "3", "46", "9", "10", "11", "12", "13", "14", "5V"],
    "R": ["43", "44", "1", "2", "42", "41", "40", "39", "38", "37",
          "36", "35", "0", "45", "48", "47", "21", "20", "19", "GND"],
}


def fab_rect(x1, y1, x2, y2):
    return [Graphic("rect", "F.Fab", (x1, y1, x2, y2), 0.1),
            Graphic("rect", "F.SilkS", (x1 - 0.1, y1 - 0.1, x2 + 0.1, y2 + 0.1), 0.12),
            Graphic("rect", "F.CrtYd", (x1 - 0.5, y1 - 0.5, x2 + 0.5, y2 + 0.5), 0.05)]


def esp_socket(used):
    pads = []
    for row, y in (("L", 0.0), ("R", -DIM["esp_row_spacing"])):
        for i, name in enumerate(ESP_PINS[row]):
            num = f"{row}{i + 1}"
            if num in used:
                pads.append(Pad(num, i * P, y, *PIN, PD))
    x1 = -DIM["esp_end_antenna"]
    x2 = 19 * P + DIM["esp_end_usb"]
    y1 = -DIM["esp_row_spacing"] - DIM["esp_side"]
    y2 = DIM["esp_side"]
    g = fab_rect(x1, y1, x2, y2)
    g.append(Graphic("text", "F.Fab", ((x1 + x2) / 2, (y1 + y2) / 2, "ESP32-S3 WROOM", 2.0)))
    g.append(Graphic("text", "F.SilkS", (x2 - 4, (y1 + y2) / 2, "USB", 1.5)))
    g.append(Graphic("line", "F.SilkS", (x1 + 6, y1 + 3, x1 + 6, y2 - 3), 0.12))
    for row, y in (("L", 0.0), ("R", -DIM["esp_row_spacing"])):
        g.append(Graphic("rect", "F.SilkS", (-P / 2, y - P / 2, 19.5 * P, y + P / 2), 0.12))
        for i in range(20):
            if f"{row}{i + 1}" not in used:
                g.append(Graphic("line", "F.Fab", (i * P - 0.5, y - 0.5, i * P + 0.5, y + 0.5), 0.1))
                g.append(Graphic("line", "F.Fab", (i * P - 0.5, y + 0.5, i * P + 0.5, y - 0.5), 0.1))
    return Footprint("Freenove_ESP32-S3_WROOM_Socket",
                     "Freenove ESP32-S3 WROOM in two 1x20 female strips; pull the contacts marked x, holes only on used pins",
                     pads, g, ref_at=(20, -12.7), value_at=(20, -9))


def shifter(unused=("A3", "B3", "GND_LV")):
    # Mounted upside down (header on the component side), so the pin order is mirrored.
    rs = DIM["shifter_row_spacing"]
    lv = ["LV", "A1", "A2", "A3", "A4", "GND_LV"]
    hv = ["HV", "B1", "B2", "B3", "B4", "GND"]
    pads = []
    for row, y in ((lv, 0), (hv, rs)):
        for i, n in enumerate(row):
            if n in unused:
                pads.append(Pad("", i * P, y, PD, PD, PD, "NP"))
            else:
                pads.append(Pad(n, i * P, y, *PIN, PD))
    g = fab_rect(-3.937, -2.54, 16.51, rs + 2.54)
    g.append(Graphic("text", "F.SilkS", (6.35, rs / 2 - 1, "UPSIDE", 1.0)))
    g.append(Graphic("text", "F.SilkS", (6.35, rs / 2 + 1, "DOWN", 1.0)))
    return Footprint("Adafruit_757_UpsideDown", "Adafruit 757 4-channel BSS138 level shifter, mounted components down",
                     pads, g, ref_at=(6.35, -4), value_at=(6.35, rs + 4))


def mp1584():
    dx, dy, m = DIM["mp1584_pad_dx"], DIM["mp1584_pad_dy"], DIM["mp1584_margin"]
    d, s = DIM["lead_drill"], 3.0
    pads = [Pad("VIN", 0, 0, s, s, d, "FB", "circle"),
            Pad("GND", 0, dy, s, s, d, "FB", "circle"),
            Pad("VOUT", dx, 0, s, s, d, "FB", "circle"),
            Pad("GND", dx, dy, s, s, d, "FB", "circle")]
    g = fab_rect(-m, -m, dx + m, dy + m)
    g.append(Graphic("text", "F.Fab", (dx / 2, dy / 2, "MP1584", 1.5)))
    g.append(Graphic("text", "F.SilkS", (2.5, 0, "IN", 1.0)))
    g.append(Graphic("text", "F.SilkS", (dx - 3.2, 0, "OUT", 1.0)))
    return Footprint("MP1584_Module", "MP1584 mini buck module, 4 corner pads on pin legs",
                     pads, g, jumpers=True, ref_at=(dx / 2, dy / 2 - 3), value_at=(dx / 2, dy / 2 + 3))


def header(n, skip=(), name=None, pad=PIN):
    pads = []
    for i in range(n):
        num = str(i + 1)
        if num in skip:
            pads.append(Pad(num, i * P, 0, PD, PD, PD, "NP"))
        else:
            pads.append(Pad(num, i * P, 0, *pad, PD, shape="rect" if i == 0 else "oval"))
    g = fab_rect(-1.27, -1.27, (n - 1) * P + 1.27, 1.27)
    return Footprint(name or f"PinHeader_1x{n:02d}_P2.54mm",
                     f"1x{n} 2.54 mm male header, pads on the solder side only",
                     pads, g, model=f"{MODELS}/Connector_PinHeader_2.54mm.3dshapes/PinHeader_1x{n:02d}_P2.54mm_Vertical.step",
                     model_rotate=(0, 0, -90), ref_at=((n - 1) * P / 2, -3), value_at=((n - 1) * P / 2, 3))


def wire_pads(n=2, ties=(True, True)):
    d, s, p = DIM["wire_drill"], DIM["wire_pad"], DIM["wire_pitch"]
    pads = [Pad(str(i + 1), i * p, 0, s, s, d, "FB", "rect" if i == 0 else "circle") for i in range(n)]
    zx = (-4.2 if ties[0] else -s / 2, (n - 1) * p + 4.2 if ties[1] else (n - 1) * p + s / 2)
    pads += [Pad("", x, 0, 2.0, 2.0, 2.0, "NP") for x, t in zip(zx, ties) if t]
    g = [Graphic("rect", "F.SilkS", (-s / 2 - 0.3, -s / 2 - 0.3, (n - 1) * p + s / 2 + 0.3, s / 2 + 0.3), 0.12),
         Graphic("rect", "F.CrtYd", (zx[0] - 1.5, -s / 2 - 0.8, zx[1] + 1.5, s / 2 + 0.8), 0.05)]
    suffix = {(True, True): "", (True, False): "_TieLeft", (False, True): "_TieRight", (False, False): "_NoTie"}[tuple(ties)]
    return Footprint(f"WirePads_1x{n:02d}{suffix}", "Solder holes for stranded wire, zip-tie holes at both ends for strain relief",
                     pads, g, ref_at=((n - 1) * p / 2, -3.5), value_at=((n - 1) * p / 2, 3.5))


def barrel_jack():
    pads = [Pad("1", 0, 0, 3.6, 4.0, (1.2, 3.2), "B", "rect"),
            Pad("2", -6, 0, 3.6, 4.0, (1.2, 3.2), "B", "oval"),
            Pad("3", -3, 4.7, 4.0, 3.6, (3.2, 1.2), "B", "oval")]
    g = fab_rect(-13.7, -4.5, 0.8, 4.5)
    return Footprint("BarrelJack_5.5x2.1_Horizontal", "DC barrel jack 5.5 x 2.1 mm, horizontal, mouth towards -x",
                     pads, g, model=f"{MODELS}/Connector_BarrelJack.3dshapes/BarrelJack_Horizontal.step",
                     ref_at=(-7, -6), value_at=(-7, 0))


def diode_do201():
    d, s = DIM["power_lead_drill"], DIM["power_lead_pad"]
    pads = [Pad("1", 0, 0, s, s, d, "FB", "rect"), Pad("2", 12.7, 0, s, s, d, "FB", "circle")]
    g = fab_rect(1.6 + 0.5, -2.6, 11.1 - 0.5, 2.6)
    g.append(Graphic("line", "F.SilkS", (3.4, -2.7, 3.4, 2.7), 0.3))
    return Footprint("D_DO-201AD_P12.70mm_Horizontal", "DO-201AD axial diode (SB560), 12.7 mm lead spacing",
                     pads, g, model=f"{MODELS}/Diode_THT.3dshapes/D_DO-201AD_P12.70mm_Horizontal.step",
                     ref_at=(6.35, -4), value_at=(6.35, 4))


def resistor():
    d, s = DIM["lead_drill"], DIM["lead_pad"]
    pads = [Pad("1", 0, 0, s, s, d, "FB", "circle"), Pad("2", 10.16, 0, s, s, d, "FB", "circle")]
    g = fab_rect(1.93, -1.25, 8.23, 1.25)
    return Footprint("R_Axial_P10.16mm_Horizontal", "Axial resistor, 0207, 10.16 mm lead spacing",
                     pads, g, model=f"{MODELS}/Resistor_THT.3dshapes/R_Axial_DIN0207_L6.3mm_D2.5mm_P10.16mm_Horizontal.step",
                     ref_at=(5.08, -2.6), value_at=(5.08, 2.6))


def cap_disc():
    d, s = DIM["lead_drill"], DIM["lead_pad"]
    pads = [Pad("1", 0, 0, s, s, d, "FB", "circle"), Pad("2", 5.0, 0, s, s, d, "FB", "circle")]
    g = fab_rect(-0.0, -1.25, 5.0, 1.25)
    return Footprint("C_Disc_P5.00mm", "Ceramic disc capacitor, 5 mm lead spacing",
                     pads, g, model=f"{MODELS}/Capacitor_THT.3dshapes/C_Disc_D5.0mm_W2.5mm_P5.00mm.step",
                     ref_at=(2.5, -2.6), value_at=(2.5, 2.6))


def cap_radial(dia, pitch):
    w, h = (2.4, 2.4) if pitch >= 3.5 else PIN
    pads = [Pad("1", 0, 0, w, h, DIM["lead_drill"], "B", "rect"),
            Pad("2", pitch, 0, w, h, DIM["lead_drill"], "B", "circle" if w == h else "oval")]
    r = dia / 2
    c = pitch / 2
    g = [Graphic("circle", "F.Fab", (c, 0, r), 0.1), Graphic("circle", "F.SilkS", (c, 0, r + 0.12), 0.12),
         Graphic("circle", "F.CrtYd", (c, 0, r + 0.5), 0.05),
         Graphic("text", "F.SilkS", (c - r - 1.2, -r + 0.6, "+", 1.5))]
    pn = f"{pitch:.2f}"
    return Footprint(f"CP_Radial_D{dia:.1f}mm_P{pn}mm", f"Radial electrolytic capacitor, {dia} mm, {pitch} mm pitch, pads on the solder side",
                     pads, g, model=f"{MODELS}/Capacitor_THT.3dshapes/CP_Radial_D{dia:.1f}mm_P{pn}mm.step",
                     ref_at=(c, -r - 1.5), value_at=(c, r + 1.5))


def cap_radial_lying(dia, length, pitch):
    pads = [Pad("1", 0, 0, 2.4, 2.4, DIM["lead_drill"], "B", "rect"),
            Pad("2", pitch, 0, 2.4, 2.4, DIM["lead_drill"], "B", "circle")]
    x1, x2 = pitch / 2 - dia / 2, pitch / 2 + dia / 2
    y1 = 1.2 + 1.5
    g = fab_rect(x1, y1, x2, y1 + length)
    g.append(Graphic("text", "F.SilkS", (-2.2, 0, "+", 1.5)))
    return Footprint(f"CP_Radial_D{dia:.1f}mm_L{length:.1f}mm_P{pitch:.2f}mm_Lying",
                     f"Radial electrolytic capacitor, {dia} x {length} mm, {pitch} mm pitch, leads bent 90 degrees so it lies flat",
                     pads, g, ref_at=(pitch / 2, y1 + length / 2 - 1.5), value_at=(pitch / 2, y1 + length / 2 + 1.5))


def mounting_hole():
    d = DIM["mount_drill"]
    return Footprint("MountingHole_M3", "M3 mounting hole, no copper",
                     [Pad("", 0, 0, d, d, d, "NP")],
                     [Graphic("circle", "F.CrtYd", (0, 0, 3.0), 0.05), Graphic("circle", "Cmts.User", (0, 0, 2.8), 0.1)],
                     ref_at=(0, -4.2), value_at=(0, 4.2))
