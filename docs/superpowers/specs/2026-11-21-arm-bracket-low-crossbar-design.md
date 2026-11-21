# Arm switch bracket with a low cross-bar

## Why

The 12 V bench jack (J2) sits at the power board's rear edge, about 15 mm right of centre, mouth backwards at y ≈ 141, hole centre about 42 mm above the floor. The current arm bracket's plate starts at y = 143 and spans the full width at z 42–46, so it is in the plug's path: a straight plug cannot go in without tilting past the plate. The switch stays at the back, reachable from behind.

## Goals

- A straight 5.5 × 2.1 mm plug goes into J2 with nothing above or behind the jack, so it can be seen and gripped from above.
- The switch stays behind the tray, lever up, reachable from behind.
- Same mounting as now: the feet sit on the tubes over the rear cradles and are held by the cradles' own ties.

Non-goals: moving J2 or any other board change.

## Approaches

- **A. Raise the plate** over the plug. Smallest change, but the lever top lands about 1 mm under the 75 mm ceiling and the plug still goes in blind under a roof.
- **B. Low cross-bar under the plug path** (picked). Clears everything above the jack and lowers the lever. The switch body hangs closer to the floor.
- **C. Lever pointing backwards** from a rear wall. Lowest, but easy to knock.

## Design (B)

- **Feet:** as now, a block over each tube at |x| 35–53 with the bore, the roof over the tube and the tie notches at the front. They run from the rear cradle (y = 143) to the bracket's rear end.
- **Legs and cross-bar:** a leg inboard of each foot drops from the foot to a cross-bar behind the tray's rear end (1 mm gap to the tray at y = 155), which spans between the legs below the plug.
  - Proposed: cross-bar top at about 34 mm (3 mm under the plug), 4 mm thick, about 15 mm deep (y 156–171), so the bracket's rear end moves from y = 167 to 171.
- **Switch:** the bushing goes up through the cross-bar on the left, mirroring the jack at x ≈ −15, nut on top, lever up. The lever top is at about 56 mm (was 68). The body hangs to about 12 mm above the floor, behind the tray.
- **Nothing above the cross-bar between the legs:** the centre plate is gone.
- **Printing:** rear face down. The bores then stand vertical and the cross-bar is a ledge at the base, with no overhangs.

### Checks in `tray.py`

- **New plug keep-out space:** the jack's x comes from `power.PARTS["J2"]`. Its height comes from a new jack-axis-above-board value plus the plug diameter with margin, and it runs from the jack mouth to the bracket's rear end plus the plug's length. The bracket, the switch body and the tray must stay out of it.
- **New floor check:** the switch body bottom must stay at least a few mm above the floor.
- **Kept:** the existing checks, that the feet fit the tubes, the bracket misses the tray and Stack 2, and the switch body misses everything.

### Docs

Update the README (the `arm-bracket.stl` row, step 6 and step 8, including the lever height) and `docs/wiring.md` (the J3 row and the layout bullet).

## Left to the builder

The exact leg width, the cross-bar thickness and depth, the jack-axis height (about 6.5 mm on the KiCad barrel-jack model) and the plug margin, within the clearances above.
