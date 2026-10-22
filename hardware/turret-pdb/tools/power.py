import parts
import stack

FP = {
    "mp1584": parts.mp1584(),
    "h3": parts.header(3),
    "h4": parts.header(4),
    "wire2_r": parts.wire_pads(2, ties=(False, True)),
    "wire2_l": parts.wire_pads(2, ties=(True, False)),
    "jack": parts.barrel_jack(),
    "diode": parts.diode_do201(),
    "res": parts.resistor(),
    "cdisc": parts.cap_disc(),
    "cp6": parts.cap_radial(6.3, 2.5),
    "cp10l": parts.cap_radial_lying(10.0, parts.DIM["c3_length"], 5.0),
    "link": stack.LINK_FP,
    "hole": parts.mounting_hole(),
}


def buck(volts, net):
    return {"VIN": ("IN+", "VIN"), "GND": ("GND", "GND"), "VOUT": ("OUT+", net)}


PARTS = {
    "U3": ("mp1584", "MP1584 7.5V", buck(7.5, "+7V5"), (41.5, 2.5, 0)),
    "U4": ("mp1584", "MP1584 6V", buck(6, "+6V"), (41.5, 21.5, 0)),
    "U5": ("mp1584", "MP1584 5V", buck(5, "+5V"), (41.5, 40.5, 0)),
    "J1": ("wire2_l", "Battery in", {"1": ("VBAT", "VBAT"), "2": ("GND", "GND")}, (14.0, 5.0, 0)),
    "J2": ("jack", "12V bench", {"1": ("+", "VJACK"), "2": ("-", "GND"), "3": ("SW", None)}, (31.0, 44.0, 90)),
    "D1": ("diode", "SB560", {"1": ("K", "VIN"), "2": ("A", "VBAT")}, (36.0, 26.0, 90)),
    "D2": ("diode", "SB560", {"1": ("K", "VIN"), "2": ("A", "VJACK")}, (29.0, 38.5, 90)),
    "C2": ("cp6", "100uF 35V", {"1": ("+", "VIN"), "2": ("-", "GND")}, (34.5, 32.0, 0)),
    "R1": ("res", "39k 1%", {"1": ("~", "VBAT"), "2": ("~", "VBAT_SENSE")}, (21.0, 11.0, 0)),
    "R2": ("res", "8.2k 1%", {"1": ("~", "VBAT_SENSE"), "2": ("~", "GND")}, (21.0, 16.0, 0)),
    "C1": ("cdisc", "100nF", {"1": ("~", "VBAT_SENSE"), "2": ("~", "GND")}, (21.0, 21.0, 0)),
    "J11": ("wire2_r", "Servo bus 7.5V", {"1": ("+7V5", "+7V5"), "2": ("GND", "GND")}, (66.0, 12.0, 270)),
    "J3": ("h4", "Arm switch", {"1": ("6V_SW", "PUSH_6V"), "2": ("6V", "+6V"), "3": ("GND", "GND"), "4": ("SENSE", "ARM_SENSE")}, (66.0, 33.5, 90)),
    "J4": ("h3", "Pusher", {"1": ("GND", "GND"), "2": ("V+", "PUSH_6V"), "3": ("SIG", "PUSH_SIG")}, (17.0, 51.0, 0)),
    "C3": ("cp10l", "1000uF 16V", {"1": ("+", "PUSH_6V"), "2": ("-", "GND")}, (17.7, 25.0, 0)),
    "J10": ("wire2_r", "Vision 5V", {"1": ("5V", "+5V"), "2": ("GND", "GND")}, (66.0, 39.0, 270)),
    "J12": stack.link(),
    **stack.holes(),
}

NETCLASSES = {
    "Power": (4.0, ["VBAT", "VJACK", "VIN", "GND", "+7V5", "+6V", "PUSH_6V"]),
    "Logic": (3.0, ["+5V"]),
}

B, F = "B.Cu", "F.Cu"
TRACKS = []
VIAS = []


def T(net, layer, width, *pts):
    TRACKS.append((net, layer, width, list(pts)))


def V(net, x, y):
    VIAS.append((net, x, y))
