# Turret controller board stack

Split the copper-tape controller board in `hardware/turret-pdb` into a power board and a logic board that stack on standoffs, and document where the quadcopter PDB and BMS go, so the turret electronics fit between the frame tubes behind the blue base.

## Goal

The current board is 100 × 95 mm. The space it has to fit is the rear of the static frame, between two 12 mm tubes about 75 mm apart. Success means:

- Every layer of the electronics is at most 72 mm wide (across the tubes).
- Nothing rises above the top of the blue base, about 75 mm above the floor, because the turret sweeps the space above it.
- The copper-tape process and its rules are unchanged: 2 mm minimum tracks, 1 mm clearance (0.55 mm at 2.54 mm-pitch pads and vias), 1.2 mm base + 0.6 mm raised copper per plate, every part soldered on the back only.

The tubes are not cut yet, so length along the tubes is free. The holders and tube clamps are designed by the owner in their own CAD once the boards exist; this work only has to give them the dimensions.

## Mechanical layout

Two stacks sit one behind the other between the tubes, behind the blue base that holds the batteries.

```
        front (barrel)                                   rear
  ┌──────────────┐   ┌─────────────┐   ┌────────────────────┐
  │  blue base   │   │  Stack 1    │   │  Stack 2           │
  │  (battery)   │   │  PDB        │   │  logic board       │ ← ESP antenna at the rear edge
  │              │   │  BMS        │   │  power board       │
  └──────────────┘   └─────────────┘   └────────────────────┘
  ══════════════════ 12 mm tubes, ~75 mm apart ══════════════════
```

**Stack 1: bought boards, nothing to design.**

| Layer | Part | Size | Mounting |
|---|---|---|---|
| Top | Quadcopter PDB with XT60 (Matek PDB-XT60 clone) | about 36 × 50 mm including the XT60 tab | 4 × M3 on a 30.5 × 30.5 mm pattern (measure before designing the holder) |
| Bottom | 4S 40 A BMS | 55 × 44 mm | No mounting holes; the holder grips its edges |

The PDB is on top so its pads stay reachable. The pack leads and the B1–B3 balance taps run from the battery covers in the blue base back to the BMS. The fuse and the XT60 sit in a short loop out of the stack between BMS and PDB; the XT60 stays reachable, since it is the power switch.

**Stack 2: the two copper-tape boards.**

- Power board at the bottom, logic board on top, same outline and same four M3 holes, on M3 standoffs.
- Board gap: 17 mm between the top face of the power board and the bottom face of the logic board. It has to clear the tallest power-board part (C2, about 12.2 mm standing on its leads) plus the logic board's pin tails on its back (about 3 mm) plus 1 mm. The gap is a constant in `stack.py`, checked by `generate.py`. (Raised from 15 mm after review.)
- Stack 2 is about 42 mm tall: 3 mm of tails under the power board, two 3.6 mm boards, the 15 mm gap and about 17 mm for the ESP in its sockets.
- Target outline: 70 mm across the tubes (hard limit 72) by about 62 mm along them. The ESP runs along the tubes with its antenna at the rear edge, so its 57 mm length sets the second dimension; routing may grow it, which the free tube length allows.
- The ESP antenna end is at the logic board's rear edge, so the antenna is above the tubes and away from the carbon. No track or via runs under the antenna end. The USB connector is at the front end of the ESP; the cable reaches it from the side through the gap between the two boards and Stack 1.

## Electrical split

GPIOs, nets and part values do not change. Only which board each part is on changes.

**Power board**

| Refs | What |
|---|---|
| J1 | Battery input wire pads (VBAT, GND), fed from one PDB pad pair |
| J2, D1, D2, C2 | 12 V bench jack, the SB560 diode OR, 100 µF on VIN |
| U3, U4, U5 | MP1584 7.5 V, 6 V and 5 V |
| R1, R2, C1 | Battery divider at the battery input, before D1 |
| J11 | 7.5 V servo bus wire pads |
| J3 | Arm switch header (PUSH_6V, +6V, GND, ARM_SENSE) |
| J4, C3 | Pusher solder holes (GND, PUSH_6V, PUSH_SIG) and its 1000 µF. C3 is mounted lying on its side, so it is 10 mm tall instead of about 20 mm. J4 was a header until review found a mated plug would not fit under the logic board |
| J10 | Vision node 5 V wire pads |

**Logic board**

| Refs | What |
|---|---|
| U1 | Freenove ESP32-S3 WROOM in sockets |
| U2 | Adafruit 757 level shifter, upside down as now |
| J5, J6 | ESC signal headers |
| J7 | TTLinker |
| J8 | TFmini Plus |
| J9, R3, R4 | Light gate header and its resistors |

**Interconnect**

Five wire pads on each board, using the existing `wire_pads` footprint at 6 mm pitch, at the same position on the same edge of both boards so each wire runs straight up:

| Order | Net | Wire | Current |
|---|---|---|---|
| 1 | GND | 22 AWG | about 0.5 A return |
| 2 | +5V | 22 AWG | about 0.5 A: ESP, TFmini, TTLinker logic, shifter HV, light gate |
| 3 | VBAT_SENSE | 26 AWG | divided battery voltage, microamps |
| 4 | ARM_SENSE | 26 AWG | pull-up current |
| 5 | PUSH_SIG | 26 AWG | 5 V servo signal |

The level shifter's 3V3 side stays on the logic board, so the 3V3 rail never leaves it. The arm switch interlock is unchanged: PUSH_6V exists only on the power board, between J3 pole A, J4 and C3.

## Tooling

The generator keeps its approach (Python describes placement and every track, `generate.py` writes KiCad files) but builds two boards.

```
hardware/turret-pdb/
  tools/
    stack.py         outline W × H, corner radius, M3 hole positions, board gap, interconnect pad positions and net order
    power.py         power board PARTS, FP, NETCLASSES, TRACKS, VIAS (replaces board.py)
    logic.py         logic board, same shape
    generate.py      generate.py [power|logic]; no argument builds both
    route_gnd.py     route_gnd.py power|logic
    parts.py, kicad.py, board_shapes.py, plates.py   unchanged except for C3's lying body outline in parts.py
    export.sh        ERC, DRC, plates, PDFs and renders for each board into out/power/ and out/logic/
  power/   turret-power.kicad_pro, .kicad_sch, .kicad_pcb, .kicad_dru, .kicad_sym, .pretty/, fp-lib-table, sym-lib-table
  logic/   turret-logic.kicad_pro, ...
```

Each project gets its own generated footprint and symbol library, because `generate.py` writes a library from the board's own parts and clears it first; a shared library would need a merge step for no gain. The single-board files at the top of `hardware/turret-pdb` (`turret-pdb.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_dru`, `.kicad_sym`, `turret-pdb.pretty/`, `fp-lib-table`, `sym-lib-table`) are removed. Both `.kicad_dru` files keep today's two rules.

`generate.py` checks, failing the build on any mismatch:

- The logic board's ESP pads against `firmware/turret/src/board.rs`, as now.
- The power board's interlock nets, as now: PUSH_6V is exactly J3.1, J4.2 and C3.1; +6V is exactly U4.VOUT and J3.2; VJACK and VBAT as today.
- ESC headers J5 and J6 have no pin 2.
- Both boards' interconnect pads are at the same XY, in the order of `stack.py`, with the same nets.
- Every net present on both boards is one of the five interconnect nets, and each of those reaches the interconnect pad on both boards.
- Outline and mounting holes come only from `stack.py`.
- No logic-board track or via passes under the ESP antenna end.

## Documentation

- `hardware/turret-pdb/README.md`: two boards and the stack; a table of the outline and M3 hole coordinates, the interconnect pad positions and the board gap for designing the holders; Stack 1 part dimensions; interconnect wiring; build and print steps per board; the "Before printing" checks, which stay.
- `docs/wiring.md`: the controller board section describes the stack; the battery section moves the BMS into Stack 1 with the pack and balance lead runs and their strain relief; the power drawing shows BMS → fuse → XT60 → PDB as the loop out of Stack 1; the PDB pads table gives the single 20 AWG feed to the power board in place of pad pairs 3–5.
- `docs/wiring.html`: kept in step with `wiring.md` where the drawings change.
- `CLAUDE.md`: the `hardware/turret-pdb` line names `power.py`, `logic.py` and `stack.py` in place of `board.py`, and the board command runs for both boards.

## Out of scope

- The printed holders and tube clamps for both stacks.
- Any change to firmware, GPIOs, nets or part values.
- Moving to a fab-house PCB.

## Verification

- `python3 tools/generate.py` passes all checks and writes both projects.
- `tools/export.sh` reports ERC 0, DRC errors 0, unconnected 0 and schematic parity 0 for both boards, and writes watertight plates for both.
- The renders of both boards show the antenna end at the logic board's rear edge, no part on the power board taller than the gap allows under the logic board, and the interconnect pads lining up.
- Both outlines are at most 72 mm across the tubes.

The generator has no Python test suite; its checks are assertions in `generate.py` that fail the build, and KiCad's ERC and DRC. New checks follow that pattern.
