import json
import math
import sys

import pcbnew

MAX_ERROR = pcbnew.FromMM(0.005)


def contours(ps):
    out = []
    for i in range(ps.OutlineCount()):
        rings = [ps.Outline(i)] + [ps.Hole(i, j) for j in range(ps.HoleCount(i))]
        out.append([[[pcbnew.ToMM(p.x), pcbnew.ToMM(p.y)] for p in r.CPoints()] for r in rings])
    return out


def mm(p):
    return [pcbnew.ToMM(p.x), pcbnew.ToMM(p.y)]


def main(path, out):
    board = pcbnew.LoadBoard(path)
    outline = pcbnew.SHAPE_POLY_SET()
    board.GetBoardPolygonOutlines(outline, True)
    holes = pcbnew.SHAPE_POLY_SET()
    pads, vias, tracks = [], [], {"front": [], "back": []}
    for fp in board.GetFootprints():
        for pad in fp.Pads():
            if pad.GetDrillSize().x > 0:
                hole = pcbnew.SHAPE_POLY_SET()
                pad.TransformHoleToPolygon(hole, 0, MAX_ERROR, pcbnew.ERROR_INSIDE)
                holes.Append(hole)
                pads.append([pad.GetNetname() or f"{fp.GetReference()}.{pad.GetNumber()}".rstrip("."), contours(hole)])
    for t in board.GetTracks():
        if isinstance(t, pcbnew.PCB_ARC):
            sys.exit(f"{path}: arc tracks are not supported by the wired plate")
        if isinstance(t, pcbnew.PCB_VIA):
            c, r = t.GetPosition(), t.GetDrillValue() / 2
            n = max(32, int(2 * math.pi * r / MAX_ERROR) // 8)
            ring = pcbnew.SHAPE_LINE_CHAIN()
            for k in range(n):
                a = 2 * math.pi * k / n
                ring.Append(int(c.x + r * math.cos(a)), int(c.y + r * math.sin(a)))
            ring.SetClosed(True)
            holes.AddOutline(ring)
            hole = pcbnew.SHAPE_POLY_SET()
            hole.AddOutline(ring)
            vias.append([t.GetNetname(), contours(hole)])
        else:
            side = {pcbnew.F_Cu: "front", pcbnew.B_Cu: "back"}[t.GetLayer()]
            tracks[side].append([t.GetNetname(), *mm(t.GetStart()), *mm(t.GetEnd())])
    layers = {}
    for name, layer in (("front", pcbnew.F_Cu), ("back", pcbnew.B_Cu)):
        ps = pcbnew.SHAPE_POLY_SET()
        board.ConvertBrdLayerToPolygonalContours(layer, ps)
        layers[name] = contours(ps)
    with open(out, "w") as f:
        json.dump({"outline": contours(outline), "holes": contours(holes), **layers,
                   "tracks": tracks, "pads": pads, "vias": vias}, f)


if __name__ == "__main__":
    main(*sys.argv[1:3])
