#!/usr/bin/env python3
import heapq
import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import generate
import stack
from generate import fp_of, nets_of, resolve
from kicad import rotate

RES = 0.25
SAFE = (1.2, 0.7, 1.2)
TIGHT = (1.0, 0.56, 1.0)
CLEAR_BODY = 0.2
VIA_R, VIA_COST, BACK_COST, TURN_COST = 1.5, 25.0, 1.4, 3.0
FLAT_BODIES = {"jack", "diode", "res", "cdisc", "cp6", "cp10", "cp10l", "h3", "h3_esc", "h4"}
PIN_BODIES = {"esp", "shifter"}
LAYERS = ("F.Cu", "B.Cu")
START = {"power": "J1.2", "logic": "J12.1"}
FIRST = {"GND": 3.0}
DIRS = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)]
board = NX = NY = X = Y = None


def init(name):
    global board, NX, NY, X, Y
    board = generate.use(name)
    NX, NY = int(stack.W / RES) + 1, int(stack.H / RES) + 1
    X, Y = np.meshgrid(np.arange(NX) * RES, np.arange(NY) * RES, indexing="ij")


def capsule(ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    ll = dx * dx + dy * dy
    t = np.clip(((X - ax) * dx + (Y - ay) * dy) / ll, 0, 1) if ll else 0
    return np.hypot(X - ax - t * dx, Y - ay - t * dy)


def rect(x1, y1, x2, y2):
    x1, x2 = min(x1, x2), max(x1, x2)
    y1, y2 = min(y1, y2), max(y1, y2)
    dx, dy = np.maximum(x1 - X, X - x2), np.maximum(y1 - Y, Y - y2)
    return np.where((dx < 0) & (dy < 0), np.maximum(dx, dy), np.hypot(np.maximum(dx, 0), np.maximum(dy, 0)))


def pad_shape(cx, cy, w, h, shape):
    if shape == "circle" or (shape == "oval" and abs(w - h) < 1e-6):
        return np.hypot(X - cx, Y - cy) - w / 2
    if shape == "oval":
        r = min(w, h) / 2
        if w > h:
            return capsule(cx - w / 2 + r, cy, cx + w / 2 - r, cy) - r
        return capsule(cx, cy - h / 2 + r, cx, cy + h / 2 - r) - r
    return rect(cx - w / 2, cy - h / 2, cx + w / 2, cy + h / 2)


class Board:
    def __init__(self):
        self.margin = {l: np.full((NX, NY), np.inf) for l in LAYERS}
        self.targets = {}
        self.centres = {}

    def block(self, field, clear, layers=LAYERS):
        for l in layers:
            np.minimum(self.margin[l], field - clear, out=self.margin[l])

    def target(self, name, field, layers, centre):
        inside = field <= -0.2
        self.targets[name] = {l: inside for l in layers}
        self.centres[name] = centre


def build(net, clear=SAFE):
    CLEAR_TRACK, CLEAR_PAD, CLEAR_EDGE = clear
    b = Board()
    for ref, (key, _, _, (x0, y0, r)) in board.PARTS.items():
        fp = fp_of(ref)
        nets = nets_of(ref)
        for i, p in enumerate(fp.pads):
            dx, dy = rotate(p.x, p.y, r)
            cx, cy = x0 + dx, y0 + dy
            if p.np:
                b.block(np.hypot(X - cx, Y - cy) - p.drill / 2, CLEAR_PAD)
                continue
            w, h = (p.w, p.h) if (r + p.angle) % 180 == 0 else (p.h, p.w)
            field = pad_shape(cx, cy, w, h, p.shape)
            if nets.get(p.number) == net:
                b.target(f"{ref}.{p.number}#{i}", field, ("B.Cu",), (cx, cy))
                b.block(field, CLEAR_PAD, ("F.Cu",))
                continue
            b.block(field, CLEAR_PAD)
        if key in FLAT_BODIES:
            for g in fp.graphics:
                if g.layer != "F.Fab":
                    continue
                if g.kind == "rect":
                    a = rotate(g.pts[0], g.pts[1], r)
                    c = rotate(g.pts[2], g.pts[3], r)
                    b.block(rect(x0 + a[0], y0 + a[1], x0 + c[0], y0 + c[1]), CLEAR_BODY, ("F.Cu",))
                elif g.kind == "circle" and g.pts[2] > 1:
                    c = rotate(g.pts[0], g.pts[1], r)
                    b.block(np.hypot(X - x0 - c[0], Y - y0 - c[1]) - g.pts[2], CLEAR_BODY, ("F.Cu",))
        if key in PIN_BODIES:
            for p in fp.pads:
                dx, dy = rotate(p.x, p.y, r)
                b.block(rect(x0 + dx - 1.27, y0 + dy - 1.27, x0 + dx + 1.27, y0 + dy + 1.27), CLEAR_BODY, ("F.Cu",))
    for n, layer, width, pts in board.TRACKS:
        if n == net:
            continue
        xy = [resolve(p, width) for p in pts]
        for a, c in zip(xy, xy[1:]):
            b.block(capsule(*a, *c) - width / 2, CLEAR_TRACK, (layer,))
    for n, x, y in board.VIAS:
        if n != net:
            b.block(np.hypot(X - x, Y - y) - VIA_R, CLEAR_PAD)
    edge = np.minimum(np.minimum(X, stack.W - X), np.minimum(Y, stack.H - Y))
    b.block(edge, CLEAR_EDGE)
    return b


def route(free, via_ok, tree, goals, costs):
    dist, prev, heap = {}, {}, []
    for (l, i, j) in tree:
        for d in range(8):
            dist[(l, i, j, d)] = 0.0
            heapq.heappush(heap, (0.0, (l, i, j, d)))
    while heap:
        c, s = heapq.heappop(heap)
        if dist.get(s, math.inf) < c:
            continue
        l, i, j, d = s
        if (l, i, j) in goals:
            path = [s]
            while s in prev:
                s = prev[s]
                path.append(s)
            return goals[(l, i, j)], path[::-1]
        steps = []
        for nd, (dx, dy) in enumerate(DIRS):
            turn = min((nd - d) % 8, (d - nd) % 8)
            if turn <= 2:
                ni, nj = i + dx, j + dy
                if 0 <= ni < NX and 0 <= nj < NY and free[l][ni, nj]:
                    cost = math.hypot(dx, dy) * costs[l] + TURN_COST * turn
                    steps.append(((l, ni, nj, nd), cost))
        if via_ok[i, j]:
            steps.append(((1 - l, i, j, d), VIA_COST))
        for ns, cost in steps:
            nc = c + cost
            if nc < dist.get(ns, math.inf):
                dist[ns] = nc
                prev[ns] = s
                heapq.heappush(heap, (nc, ns))
    return None, None


def emit(path, width):
    tracks, vias = [], []
    run = [path[0]]
    for s in path[1:]:
        if s[0] != run[-1][0]:
            tracks.append((run[-1][0], run))
            vias.append((s[1] * RES, s[2] * RES))
            run = [s]
        else:
            run.append(s)
    tracks.append((run[-1][0], run))
    out = []
    for l, run in tracks:
        cells = [(s[1], s[2]) for s in run]
        if len(cells) < 2:
            continue
        pts = [cells[0]]
        for a, b, c in zip(cells, cells[1:], cells[2:]):
            if (b[0] - a[0], b[1] - a[1]) != (c[0] - b[0], c[1] - b[1]):
                pts.append(b)
        pts.append(cells[-1])
        out.append((LAYERS[l], width, [(round(i * RES, 2), round(j * RES, 2)) for i, j in pts]))
    return out, vias


def snap(pts, centre):
    if not centre or len(pts) < 2:
        return
    (ax, ay), (bx, by) = pts[-2], pts[-1]
    cx, cy = (round(c, 2) for c in centre)
    if ax == bx and abs(bx - cx) < RES:
        pts[-2], pts[-1] = (cx, ay), (cx, by)
    elif ay == by and abs(by - cy) < RES:
        pts[-2], pts[-1] = (ax, cy), (bx, cy)


def own_copper(net):
    cells = set()
    for n, layer, width, pts in board.TRACKS:
        if n == net:
            xy = [resolve(p, width) for p in pts]
            for a, c in zip(xy, xy[1:]):
                ii, jj = np.nonzero(capsule(*a, *c) <= width / 2 - RES)
                cells |= {(LAYERS.index(layer), i, j) for i, j in zip(ii, jj)}
    for n, x, y in board.VIAS:
        if n == net:
            ii, jj = np.nonzero(np.hypot(X - x, Y - y) <= VIA_R - RES)
            cells |= {(li, i, j) for li in range(len(LAYERS)) for i, j in zip(ii, jj)}
    return cells


def route_net(name, net):
    width = FIRST.get(net, max([w for w, ns in board.NETCLASSES.values() if net in ns] or [2.0]))
    passes = [(width, SAFE)] * (width > 2.0) + [(2.0, SAFE), (2.0, TIGHT)]
    costs = (1.0, BACK_COST) if net == "GND" else (BACK_COST, 1.0)
    boards = {SAFE: build(net, SAFE), TIGHT: build(net, TIGHT)}
    b = boards[SAFE]
    first = START[name] if net == "GND" else next(f"{r}.{p}" for r in board.PARTS for p, n in nets_of(r).items() if n == net)
    start = next(k for k in b.targets if k.startswith(first + "#"))
    tree = own_copper(net)
    for li, l in enumerate(LAYERS):
        if l in b.targets[start]:
            ii, jj = np.nonzero(b.targets[start][l])
            tree |= {(li, i, j) for i, j in zip(ii, jj)}
    pending = {k: v for k, v in b.targets.items() if k != start}
    tracks, vias = [], []
    for width, clear in passes:
        b = boards[clear]
        while pending:
            free, goals = [], {}
            for li, l in enumerate(LAYERS):
                ok = b.margin[l] >= width / 2
                for k, masks in pending.items():
                    if l in masks:
                        ok |= masks[l]
                for (tl, i, j) in tree:
                    if tl == li:
                        ok[i, j] = True
                free.append(ok)
                for k, masks in pending.items():
                    if l in masks:
                        ii, jj = np.nonzero(masks[l] & ok)
                        goals.update({(li, i, j): k for i, j in zip(ii, jj)})
            via_ok = (b.margin["F.Cu"] >= VIA_R) & (b.margin["B.Cu"] >= VIA_R)
            hit, path = route(free, via_ok, tree, goals, costs)
            if not hit:
                break
            t, v = emit(path, width)
            if t:
                snap(t[-1][2], b.centres.get(hit))
            tracks += t
            vias += v
            tree |= {(s[0], s[1], s[2]) for s in path}
            print(f"# {net}: {hit} at {width} mm, {len(v)} vias", file=sys.stderr)
            del pending[hit]
    for k in pending:
        print(f"# {net}: unreached {k}", file=sys.stderr)
    for layer, width, pts in tracks:
        board.TRACKS.append((net, layer, width, pts))
        print(f'T("{net}", {layer[0]}, {width}, ' + ", ".join(f"({x}, {y})" for x, y in pts) + ")")
    for x, y in vias:
        board.VIAS.append((net, round(x, 2), round(y, 2)))
        print(f'V("{net}", {round(x, 2)}, {round(y, 2)})')
    return not pending


def main(name, nets):
    init(name)
    ok = [route_net(name, net) for net in nets]
    sys.exit(0 if all(ok) else 1)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2:] or ["GND"])
