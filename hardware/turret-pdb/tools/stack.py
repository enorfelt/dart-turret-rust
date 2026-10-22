import parts
from kicad import Graphic

W, H = 70.0, 58.0
CORNER = 4.0
ORIGIN = (40.0, 40.0)
GAP = 15.0
HOLES = ((4.0, 4.0), (W - 4.0, 4.0), (4.0, H - 4.0), (W - 4.0, H - 4.0))

LINK_NETS = ("GND", "+5V", "VBAT_SENSE", "ARM_SENSE", "PUSH_SIG")
LINK_NAMES = ("GND", "5V", "VBAT", "ARM", "PUSH")
LINK_AT = (12.0, 55.0, 90)


def link_fp():
    fp = parts.wire_pads(len(LINK_NETS), ties=(False, False))
    p = parts.DIM["wire_pitch"]
    fp.graphics += [Graphic("text", "F.SilkS", (i * p, 3.4, n, 1.0)) for i, n in enumerate(LINK_NAMES)]
    fp.value_at = ((len(LINK_NETS) - 1) * p / 2, 5.4)
    return fp


LINK_FP = link_fp()


def link():
    pins = {str(i + 1): (name, net) for i, (name, net) in enumerate(zip(LINK_NAMES, LINK_NETS))}
    return ("link", "Stack link", pins, LINK_AT)


def holes():
    return {f"H{i + 1}": ("hole", "M3", {}, (x, y, 0)) for i, (x, y) in enumerate(HOLES)}
