import math
import uuid
from dataclasses import dataclass, field

NAMESPACE = uuid.UUID("6f0c4c55-8f7e-4f8a-9f37-5d2b7e1a0c42")


def uid(*parts):
    return str(uuid.uuid5(NAMESPACE, "/".join(str(p) for p in parts)))


class Atom(str):
    pass


def num(v):
    if isinstance(v, bool):
        return "yes" if v else "no"
    if isinstance(v, int):
        return str(v)
    s = f"{v:.4f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def quote(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


def sexpr(node, indent=0):
    if isinstance(node, Atom):
        return str(node)
    if isinstance(node, str):
        return quote(node)
    if isinstance(node, (int, float, bool)):
        return num(node)
    head, *rest = node
    if all(not isinstance(r, list) for r in rest):
        return "(" + " ".join([str(head)] + [sexpr(r) for r in rest]) + ")"
    pad = "\t" * (indent + 1)
    out = "(" + str(head)
    for r in rest:
        if isinstance(r, list):
            out += "\n" + pad + sexpr(r, indent + 1)
        else:
            out += " " + sexpr(r)
    return out + "\n" + "\t" * indent + ")"


def S(head, *args):
    return [Atom(head), *args]


def A(word):
    return Atom(word)


def rotate(x, y, deg):
    a = math.radians(deg)
    c, s = math.cos(a), math.sin(a)
    return x * c + y * s, -x * s + y * c


@dataclass
class Pad:
    number: str
    x: float
    y: float
    w: float
    h: float
    drill: float | tuple
    side: str = "B"
    shape: str = "oval"
    angle: float = 0

    @property
    def np(self):
        return self.side == "NP"


@dataclass
class Graphic:
    kind: str
    layer: str
    pts: tuple
    width: float = 0.12


@dataclass
class Footprint:
    name: str
    descr: str
    pads: list
    graphics: list = field(default_factory=list)
    model: str | None = None
    model_offset: tuple = (0, 0, 0)
    model_rotate: tuple = (0, 0, 0)
    jumpers: bool = False
    ref_at: tuple = (0, -3)
    value_at: tuple = (0, 3)

    def pad(self, number):
        return [p for p in self.pads if p.number == number]


def pad_layers(side):
    if side == "B":
        return ["*.Cu", "B.Mask"]
    return ["*.Cu", "*.Mask"]


def pad_node(p, rot, net=None, key=()):
    if p.np:
        node = S("pad", "", A("np_thru_hole"), A("circle"),
                 S("at", p.x, p.y), S("size", p.drill, p.drill), S("drill", p.drill),
                 S("layers", "*.Cu", "*.Mask"))
    else:
        if isinstance(p.drill, tuple):
            drill = S("drill", A("oval"), p.drill[0], p.drill[1])
        else:
            drill = S("drill", p.drill)
        at = S("at", p.x, p.y) if not (rot + p.angle) % 360 else S("at", p.x, p.y, (rot + p.angle) % 360)
        node = S("pad", p.number, A("thru_hole"), A(p.shape), at, S("size", p.w, p.h), drill,
                 S("layers", *pad_layers(p.side)))
        if p.shape == "roundrect":
            node.append(S("roundrect_rratio", 0.25))
        if p.side == "B":
            node.append(S("zone_connect", 0))
        if net:
            node.append(S("net", net))
    node.append(S("uuid", uid("pad", *key, p.number, p.x, p.y)))
    return node


def graphic_node(g, key):
    stroke = S("stroke", S("width", g.width), S("type", A("solid")))
    u = S("uuid", uid("gr", *key, g.kind, g.layer, *g.pts))
    if g.kind == "line":
        (x1, y1, x2, y2) = g.pts
        return S("fp_line", S("start", x1, y1), S("end", x2, y2), stroke, S("layer", g.layer), u)
    if g.kind == "rect":
        (x1, y1, x2, y2) = g.pts
        return S("fp_rect", S("start", x1, y1), S("end", x2, y2), stroke, S("fill", A("no")),
                 S("layer", g.layer), u)
    if g.kind == "circle":
        (cx, cy, r) = g.pts
        return S("fp_circle", S("center", cx, cy), S("end", cx + r, cy), stroke, S("fill", A("no")),
                 S("layer", g.layer), u)
    if g.kind == "text":
        (x, y, text, size) = g.pts
        return S("fp_text", A("user"), text, S("at", x, y), S("layer", g.layer), u,
                 S("effects", S("font", S("size", size, size), S("thickness", size * 0.15))))
    raise ValueError(g.kind)


def text_effects(size=1.0, hide=False):
    e = S("effects", S("font", S("size", size, size), S("thickness", size * 0.15)))
    return e


def footprint_node(fp, lib, ref="REF**", value=None, at=None, nets=None, path=None, sheetfile=None):
    rot = at[2] if at else 0
    key = (ref, fp.name)
    node = S("footprint", f"{lib}:{fp.name}" if at else fp.name)
    if not at:
        node += [S("version", 20260206), S("generator", "turret-pdb-gen"), S("generator_version", "10.0")]
    node.append(S("layer", "F.Cu"))
    if at:
        node.append(S("uuid", uid("fp", ref)))
        node.append(S("at", at[0], at[1], rot) if rot else S("at", at[0], at[1]))
    node.append(S("descr", fp.descr))
    props = [("Reference", ref, fp.ref_at, "F.SilkS", False),
             ("Value", value or fp.name, fp.value_at, "F.Fab", False),
             ("Datasheet", "", (0, 0), "F.Fab", True),
             ("Description", fp.descr, (0, 0), "F.Fab", True)]
    for name, val, (px, py), layer, hide in props:
        p = S("property", name, val, S("at", px, py, rot) if rot else S("at", px, py), S("layer", layer))
        if hide:
            p.append(S("hide", A("yes")))
        p += [S("uuid", uid("prop", *key, name)), text_effects()]
        node.append(p)
    if path:
        node += [S("path", path), S("sheetname", "/"), S("sheetfile", sheetfile)]
    node.append(S("attr", A("through_hole")))
    node.append(S("duplicate_pad_numbers_are_jumpers", A("yes" if fp.jumpers else "no")))
    for g in fp.graphics:
        node.append(graphic_node(g, key))
    for p in fp.pads:
        node.append(pad_node(p, rot, (nets or {}).get(p.number), key))
    node.append(S("embedded_fonts", A("no")))
    if fp.model:
        node.append(S("model", fp.model,
                      S("offset", S("xyz", *fp.model_offset)),
                      S("scale", S("xyz", 1, 1, 1)),
                      S("rotate", S("xyz", *fp.model_rotate))))
    return node
