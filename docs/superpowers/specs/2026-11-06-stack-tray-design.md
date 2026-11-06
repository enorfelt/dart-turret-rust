# Stack tray

A printed tray that hangs from the two frame tubes behind the blue base and holds both electronics stacks: Stack 1 (BMS and quadcopter PDB) at the front and Stack 2 (the power and logic boards from `hardware/turret-pdb`) behind it.

## Goal

The [board stack spec](2026-10-22-turret-board-stack-design.md) left the holders to be designed later. Success means:

- One tray and one deck, printed in PLA without supports, fastened with M3 screws, nuts and washers only. The tray has no nut traps.
- The tray hangs under the two 12 mm tubes in cradles, held up by two zip ties per cradle. Nothing has to flex, so PLA does not crack.
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
 ~68           ┌XT60┐               ~65 ESP top
 47            ├─PDB──┐ pillars       ┌──── logic board ───┐ 46
 42 ═══════════╪══════╪══════════════════════════════════════ tube top
 30 ═══════════╪deck══╪══════════════════════════════════════ tube bottom
                │ BMS  │              ┌──── power board ───┐ 27
 23 ┌───────────┴──────┴──────────────┴────────────────────┴──┐ floor top
 20 └─────────────────────────────────────────────────────────┘ floor bottom
    y=0   Stack 1    63  USB gap  73      Stack 2         141   155
```

The tray hangs 10 mm below the tubes, because a floor resting on the tubes would put the top of Stack 2 about 11 mm above the ceiling.

A hook that wraps over a tube from below cannot be printed flat without supports: its free lip starts in mid air. So the tray carries the tubes in open cradles instead, and zip ties over the tubes carry the weight. Two 4.8 mm ties per cradle hold far more than the tray weighs.

### Tray

- **Floor:** 3 mm, z 20–23, x ±51 (under both tubes), y 0–155.
- **Cradles:** three pairs, at y 0–12, 60–72 and 143–155, where no board sits next to the tubes, and the BMS starts at least 1 mm behind the front cradle so its tie tunnels open beside nothing. Each is a block from the floor up to the tube centre (z 36), x ±(33–51), with a 12.2 mm bowl the tube lies in, so its sides locate the tube both ways. Two tie tunnels, 5 mm along y and 2 mm tall at z 25–27, run across each block under the bowl, at y 3 and 9 within the cradle. A tie goes in at the block's inner face, through the tunnel, up the outer face, over the tube and back down. The ties top out at about z = 43.5.
- **Stack 2 bosses:** four, at the `stack.py` hole positions with the board's front-left corner at (x −35, y 73): 7 mm across, 4 mm tall (z 23–27), with a 3.4 mm hole through the boss and floor. An M3 × 12 screw from under the tray goes up into the female thread of each 17 mm female–female standoff, clamping the power board. Stack 2 is built complete before it goes on the tray, because its link wires are soldered on the power board's back. The power board's underside sits at z = 27, below the tubes, with room for its pin tails.
- **BMS pocket:** 61.6 × 46.6 mm, 1 mm deep (z 22–23), centred on x = 0, y = 36, long side across the tubes. The BMS sits on 1 mm double-sided foam tape in it, which holds it and damps vibration. Three 4 × 40 mm vent slots through the floor at y 26, 36 and 46.
- **Deck posts:** four, 7 mm across, z 23–30, at x ±20, y 8.5 and 63.5, just in front of and behind the BMS, with a 3.4 mm hole through post and floor. An M3 × 16 screw from under the tray goes up through the post and the deck and takes a washer and nut on top of the deck, so the deck comes off from above.
- **Tie-down slots:** pairs of 2.5 × 5 mm slots through the floor, 8 mm apart: one pair at x ±4, y 68 in the USB gap for the fuse holder and the 14 AWG loop, which is also where the ESP's USB cable comes in from the side; one pair at each BMS pad end, at x ±37, y 32 and 40.
- Nothing on the tray is near the ESP antenna except the rear cradles beside the tubes.

### Deck

- A flat 3 mm bar, x ±24, y 5–67, z 30–33, on the four posts. Its underside is about 2 mm above the MOSFETs (BMS top at about z = 28 on the foam), it keeps the PDB's underside away from the BMS, and it leaves both BMS pad ends open. Three vent slots over the MOSFETs, matching the floor's.
- Four pillars, 10 mm across, z 33–47, on a 30.25 mm square centred on (0, 36), with 3.6 mm holes, so an M3 screw goes through on a PDB with either a 30 or a 30.5 mm pattern. A sideways slot for an M3 nut, 5.9 × 2.6 mm, opening outwards and ending 3.3 mm past the screw axis so the nut's leading corner stops it centred, sits 3 mm below each top; M3 × 8 screws hold the PDB. The PDB's underside is at z = 47, 3.5 mm above the zip ties.
- The PDB's XT60 tab points to +x, out over the tube between the front and middle cradles, so the mated plug clears the tube and is reached from the side. Its top lands at about z = 68.
- The ⊕/⊖ leads and the fuse loop run in the 14 mm between deck and PDB, and the battery-side XT60 plug comes back in on the +x side, where P+ and P− are.
- Prints bar down, pillars up.

## Tooling

`hardware/turret-pdb/tools/tray.py`, run with `tools/.venv/bin/python` (manifold3d, as `plates.py`), writes `out/tray/tray.stl`, `out/tray/deck.stl` and `out/tray/cradle-test.stl` (one cradle with its piece of floor, to try the fit and the ties on a tube offcut). Every frame and part dimension above is in one `TRAY` dict at the top, like `DIM` in `parts.py`. It imports `HOLES`, `W` and `H` from `stack.py` for the Stack 2 bosses and the board outline. `export.sh` runs it after the boards when manifold3d is available.

Before writing anything, `tray.py` checks and stops on failure:

- Every part is watertight and in one piece.
- A failed check also removes the tray STLs left from an earlier run, so a stale file is never printed.
- The tubes fit the cradles, and nothing of the tray is in the space Stack 2 takes.
- The tops of the ESP (logic board underside from `stack.GAP`, plus the board and 17 mm of ESP on its sockets), the PDB with its XT60, and the zip ties are all below 75 mm.
- The PDB's underside, and so the XT60, clears the zip ties over the tubes.
- The deck's underside is at least 1.5 mm above the BMS MOSFETs.
- The Stack 2 outline at the power board's height fits between the tubes, and the logic board clears the tube tops.
- The Stack 2 bosses sit on the `stack.py` holes.

## Documentation

- `hardware/turret-pdb/README.md`: Stack 1 sizes corrected to the measured BMS and PDB; a "Stack tray" section with what to print, the hardware list (4 × M3 × 16, 4 × M3 × 12, 4 × M3 × 8, 8 M3 nuts, washers, six 4.8 mm zip ties for the tubes and a few for the wires, 1 mm double-sided foam tape) and assembly order (screws up through the floor, BMS on foam tape and its wires, deck, PDB, Stack 2, then hold the tray under the tubes and fit the ties). With PLA, washers under every screw head and nut, and screws only snug, as PLA creeps under constant pressure.
- `docs/wiring.md`: "Where it sits" and the BOM point to the tray; the BMS size is corrected.

## Out of scope

- Mounting the ESCs, the fuse holder in a printed clip, or a cover over the stacks.
- Changing the boards or their stack heights.

## Verification

- `tools/.venv/bin/python tools/tray.py` passes its checks and writes the three STLs.
- Opening the STLs in Bambu Studio shows them printable flat on the floor without supports.
- The cradle test takes a 12 mm tube offcut and sits tight on it with its two ties, before the full tray is printed.
