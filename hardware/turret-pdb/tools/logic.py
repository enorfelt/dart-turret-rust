import parts
import stack

ESP_USED = {
    "L1": "+3V3", "L5": "TF_TX", "L6": "TF_RX", "L9": "ESC_L", "L10": "ESC_R",
    "L19": "BUS_TX_3V3", "L20": "+5V",
    "R3": "VBAT_SENSE", "R5": "GATE", "R6": "PUSH_SIG_3V3", "R16": "ARM_SENSE",
    "R17": "BUS_RX_3V3", "R20": "GND",
}

SHIFTER = {
    "LV": "+3V3", "HV": "+5V", "GND_LV": "GND",
    "A1": "PUSH_SIG_3V3", "B1": "PUSH_SIG",
    "A2": "BUS_RX_3V3", "B2": "BUS_RX",
    "A4": "BUS_TX_3V3", "B4": "BUS_TX",
}

FP = {
    "esp": parts.esp_socket(ESP_USED),
    "shifter": parts.shifter(unused=("A3", "B3", "GND")),
    "h3": parts.header(3),
    "h3_esc": parts.header(3, skip=("2",), name="PinHeader_1x03_P2.54mm_ESC", pad=parts.DIM["pin_pad_single"]),
    "h4": parts.header(4),
    "res": parts.resistor(),
    "link": stack.LINK_FP,
    "hole": parts.mounting_hole(),
}


def esp_pins():
    return {p: (parts.ESP_PINS[p[0]][int(p[1:]) - 1], n) for p, n in ESP_USED.items()}


PARTS = {
    "U1": ("esp", "Freenove ESP32-S3 WROOM", esp_pins(), (49.8, 50.5, 90)),
    "U2": ("shifter", "Adafruit 757", {k: (k, v) for k, v in SHIFTER.items()}, (19.5, 10.5, 270)),
    "J5": ("h3_esc", "ESC left", {"1": ("GND", "GND"), "3": ("SIG", "ESC_L")}, (56.0, 31.0, 90)),
    "J6": ("h3_esc", "ESC right", {"1": ("GND", "GND"), "3": ("SIG", "ESC_R")}, (61.0, 31.0, 90)),
    "J7": ("h4", "TTLinker", {"1": ("5V", "+5V"), "2": ("GND", "GND"), "3": ("TXD", "BUS_RX"), "4": ("RXD", "BUS_TX")}, (4.0, 32.0, 270)),
    "J8": ("h4", "TFmini Plus", {"1": ("5V", "+5V"), "2": ("GND", "GND"), "3": ("TX", "TF_TX"), "4": ("RX", "TF_RX")}, (56.0, 43.5, 90)),
    "J9": ("h3", "Light gate", {"1": ("LED", "GATE_LED"), "2": ("OUT", "GATE"), "3": ("GND", "GND")}, (61.0, 43.5, 90)),
    "R3": ("res", "150", {"1": ("~", "+3V3"), "2": ("~", "GATE_LED")}, (66.0, 48.0, 90)),
    "R4": ("res", "10k", {"1": ("~", "+3V3"), "2": ("~", "GATE")}, (66.0, 32.0, 90)),
    "J12": stack.link(),
    **stack.holes(),
}

NETCLASSES = {
    "Power": (4.0, ["GND"]),
    "Logic": (3.0, ["+5V"]),
}

B, F = "B.Cu", "F.Cu"
TRACKS = []
VIAS = []


def T(net, layer, width, *pts):
    TRACKS.append((net, layer, width, list(pts)))


def V(net, x, y):
    VIAS.append((net, x, y))
