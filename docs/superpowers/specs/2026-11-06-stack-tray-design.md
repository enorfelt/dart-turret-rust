# Stack tray

A printed tray that hangs from the two frame tubes behind the blue base and holds both electronics stacks: Stack 1 (BMS and quadcopter PDB) at the front and Stack 2 (the power and logic boards from `hardware/turret-pdb`) behind it.

## Goal

The [board stack spec](2026-10-22-turret-board-stack-design.md) left the holders to be designed later. Success means:

- One tray and one deck, printed in PETG without supports, fastened with M3 screws and nuts only.
- The tray snaps onto the two 12 mm tubes and is backed up by a zip tie at each saddle.
- Nothing rises above 75 mm from the floor, where the turret sweeps.
- The BMS pads and the PDB pads stay reachable for soldering with everything mounted, and the XT60 stays reachable from the side as the power switch.
- The Stack 2 mounting holes come from `tools/stack.py`, so a board change moves them.

## Measured frame and parts

| What | Value |
|---|---|
| Frame tubes | 12 mm, 72 mm clear between them (centres at x = ±42), bottoms 30 mm above the floor |
| Space above | Nothing above 75 mm; nothing outside the tubes behind the blue base |
| Tube length | Free, cut to suit |
| BMS | 4S 40 A, 61 × 46 mm, about 5 mm thick (1.6 mm board, tallest MOSFET 3.35 mm), flat underneath, no mounting holes. Pads on the two 46 mm ends: B3 (12.6 V), B2 (8.4 V) and B− (0 V) on one; B+ (16.8 V), P+, P− and B1 (4.2 V) on the other |
| PDB | Matek PDB-XT60 or clone, 1.6 mm, flat underneath, 4 × M3 on about 30 × 30 mm. The XT60 lies on the tab with its mating face pointing out horizontally and stands about 19 mm above the board |

## Coordinates

x across the tubes (0 on the centre line, + towards the XT60 side), y along the tubes from the rear face of the blue base towards the rear, z up from the floor.

## Layout

```
 side view (z)
 75 ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ceiling
 ~67           ┌XT60┐               ~67 ESP top
 ~46           ├─PDB──┐ pillars       ┌──── logic board ───┐ ~46
 42 ═══════════╪══════╪══════════════════════════════════════ tube top
 30 ═══════════╪deck══╪══════════════════════════════════════ tube bottom
                │ BMS  │              ┌──── power board ───┐ 27
 23 ┌───────────┴──────┴──────────────┴────────────────────┴──┐ floor top
 20 └─────────────────────────────────────────────────────────┘ floor bottom
    y=0   Stack 1    57  USB gap  68      Stack 2         136 140
```

The tray hangs 10 mm below the tubes, because a floor resting on the tubes would put the top of Stack 2 about 11 mm above the ceiling.

### Tray

- **Floor:** 3 mm, underside at z = 20, about 102 mm across (x = ±51, under both tubes) and 140 mm long.
- **Side walls:** 3 mm, outside the tubes, from the floor up to the saddles. Nothing of the tray sits between a tube and the boards, which have 1 mm per side there.
- **Saddles:** three pairs at y ≈ 5, 62 and 135, each 10 mm long. Each wraps about 240° of a 12.2 mm bore with a 2.5 mm wall, open towards the floor and the centre line, so it snaps up onto the tube from below and the tube rests in its crown. The crown tops out at about z = 44.5. Each saddle has a slot for a 3–5 mm zip tie around the tube.
- **Stack 2 bosses:** four, at the `stack.py` hole positions with the board's front-left corner at (x −35, y 68). Each is 4 mm tall, 7 mm across, with an M3 clearance hole and a hex nut trap in the floor underneath. The power board's underside sits at z = 27, below the tubes, with room for its pin tails.
- **BMS pocket:** 61.6 × 46.6 mm, 1 mm deep, centred on x = 0 at y 7–54, long side across the tubes. A vent slot runs under each MOSFET row.
- **Deck posts:** four, at x = ±20, y 3 and y 57, just in front of and behind the BMS, up to the deck's underside, each with an M3 clearance hole and a nut trap in the floor.
- **Zip-tie points:** in the USB gap (y 58–68), which is also where the ESP's USB cable comes in from the side, for the fuse holder and the 14 AWG loop, and on the side walls next to the BMS pad ends.
- The rear edge carries nothing near the ESP antenna.

### Deck

- A 3 mm bar about 46 mm across (x = ±23) and long enough in y to reach the posts. It presses the middle of the BMS's two long edges, with its underside about 2 mm above the MOSFETs (z ≈ 30), and leaves both pad ends open.
- Vent slots over the MOSFETs.
- Four pillars on a 30.5 mm square, centred on x = 0 over the BMS, with 3.4 mm holes so a 30 or 30.5 mm PDB fits, and a sideways M3 nut slot near each top. They hold the PDB's underside at z = 46, above the saddle crowns.
- The PDB's XT60 tab points to +x, out over the tube between the front and middle saddles, so the mated plug clears the tube and is reached from the side. Its top lands at about z = 67.
- The ⊕/⊖ leads and the fuse loop run in the 13 mm under the PDB, and the battery-side XT60 plug comes back in on the +x side, where P+ and P− are.

## Tooling

`hardware/turret-pdb/tools/tray.py`, run with `tools/.venv/bin/python` (manifold3d, as `plates.py`), writes `out/tray/tray.stl`, `out/tray/deck.stl` and `out/tray/saddle-test.stl` (one saddle on a short piece of wall, to try the snap on a tube offcut). Every frame and part dimension above is in one `TRAY` dict at the top, like `DIM` in `parts.py`. It imports `HOLES`, `W` and `H` from `stack.py` for the Stack 2 bosses and the board outline. `export.sh` runs it after the boards when manifold3d is available.

Before writing anything, `tray.py` checks and stops on failure:

- Every part is watertight and in one piece.
- The tops of the ESP (power board underside plus the Stack 2 height above it), the PDB with its XT60, and the saddles are all below 75 mm.
- The XT60's underside clears the saddle crowns.
- The deck's underside clears the BMS MOSFETs.
- The Stack 2 outline at the power board's height fits between the tubes, and the logic board clears the tube tops.
- The Stack 2 bosses sit on the `stack.py` holes.

## Documentation

- `hardware/turret-pdb/README.md`: Stack 1 sizes corrected to the measured BMS and PDB; a "Stack tray" section with what to print, the hardware list (M3 screws and nuts, zip ties) and assembly order (BMS and its wires, deck, PDB, Stack 2, then snap onto the tubes).
- `docs/wiring.md`: "Where it sits" and the BOM point to the tray; the BMS size is corrected.

## Out of scope

- Mounting the ESCs, the fuse holder in a printed clip, or a cover over the stacks.
- Changing the boards or their stack heights.

## Verification

- `tools/.venv/bin/python tools/tray.py` passes its checks and writes the three STLs.
- Opening the STLs in Bambu Studio shows them printable flat on the floor without supports.
- The saddle test snaps onto a 12 mm tube offcut and holds before the full tray is printed.
