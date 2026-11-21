# Arm Bracket Low Cross-bar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the arm switch bracket so a straight plug reaches the 12 V jack (J2), with the switch on a low cross-bar behind the tray.

**Architecture:** All geometry lives in `hardware/turret-pdb/tools/tray.py` (manifold3d), with its dimensions in `TRAY` and its checks in `check_envelope`/`check_parts`. The bracket keeps its feet over the tubes, drops the centre plate, and gains two legs and a cross-bar below a new plug keep-out space derived from `power.PARTS["J2"]`.

**Tech Stack:** Python 3, manifold3d, unittest (`.venv` in `hardware/turret-pdb/tools`).

**Spec:** `docs/superpowers/specs/2026-11-21-arm-bracket-low-crossbar-design.md`

## Global Constraints

- No code comments by default (CLAUDE.md).
- Ceiling 75 mm above the floor; z = 0 is the floor.
- Feet and tie notches unchanged; ties still go over the feet at the rear cradles.
- Run commands from `hardware/turret-pdb/tools`: `.venv/bin/python -m unittest test_tray && .venv/bin/python tray.py`.

## Review Focus

- A plug held slightly high or drooping: the keep-out space is a box from 6 mm under the jack axis up to the ceiling, not just a cylinder (pinned by `test_plug_path_open_to_the_ceiling`).
- The cross-bar raised by someone later: `arm_bar_top` above the plug bottom must be flagged (`test_high_bar_flagged`).
- The switch body near the floor: flagged under `min_floor_clear` (`test_switch_body_near_floor_flagged`).
- The jack moved on the power board: the plug space follows `power.PARTS["J2"]` (`test_plug_space_follows_j2`).
- The printed orientation: rear face flat on the bed, so feet and cross-bar both touch it (`test_prints_on_its_rear_face`).

---

### Task 1: Bracket geometry, plug and floor checks

**Files:**
- Modify: `hardware/turret-pdb/tools/tray.py` (`TRAY`, `envelope`, `check_envelope`, `check_parts`, `switch_centre`, `switch_space`, `arm_bracket`, `arm_bracket_print`; new `plug_space`, `jack_xz`, `arm_bar`)
- Test: `hardware/turret-pdb/tools/test_tray.py` (`Envelope`, `ArmBracket`)

**Interfaces:**
- Produces: `jack_xz(t) -> (float, float)` (jack axis x and z in tray coordinates), `plug_space(t=TRAY) -> Manifold`, `arm_bar(t) -> (y0, y1, z0, z1)`; `envelope()["switch_top"]` and new `envelope()["switch_bottom"]`.

`TRAY` changes: remove `arm_plate_t`; `arm_rear` 12.0 → 16.0; `switch_x` 0.0 → -15.0; add `arm_leg_w` 4.0, `arm_bar_gap` 1.0, `arm_bar_top` 34.0, `arm_bar_t` 4.0, `jack_axis` 6.5, `plug_d` 12.0, `min_floor_clear` 5.0.

Derived values the tests pin: arm span y 143–171; bar y 156–171, z 30–34; switch centre (-15, 163.5); switch body z 12–30; `switch_top` 56.0; jack axis (15.0, 42.1); plug space x 9–21, y 141–172, z 36.1–75.

- [ ] **Step 1: Update and add tests in `test_tray.py`**
  - `Envelope.test_heights`: `"switch_top": 56.0`, add `"switch_bottom": 12.0`.
  - `test_switch_lever_above_ceiling`: `switch_above=42.0`.
  - New `test_switch_body_near_floor_flagged`: `assertFlags("floor", switch_body=(13.0, 13.0, 26.0))`.
  - `ArmBracket.test_spans_the_rear_cradles_and_reaches_behind_the_tray`: bounding box `(-53, y0, 30.0, 53, length + 16, 46.1)`.
  - Replace `test_switch_hole_in_the_middle_behind_the_tray` with `test_switch_hole_on_the_bar_left_of_the_jack`: `sx == -15.0`, `sy == 163.5`, not solid at `(sx + 3.0, sy, 32)`, solid at `(sx + 3.4, sy, 32)`, `sy - switch_hole_d / 2 > length + arm_bar_gap`.
  - New `test_nothing_above_the_bar_between_the_legs`: not solid at `(x, 163.5, z)` for x in (-20, 0, 15, 25), z in (36, 40, 45).
  - New `test_legs_join_feet_to_bar`: solid at `(±33, 163.5, z)` for z in (32, 40, 45).
  - Replace `test_deep_switch_body_flagged` with `test_high_bar_flagged`: `t = dict(TRAY, arm_bar_top=38.0)`; `check_parts(t, {"tray": tray(t), "arm-bracket": arm_bracket(t)})` contains a problem with "plug".
  - New `test_plug_space_follows_j2`: `jack_xz(TRAY) == (15.0, 42.1)` (places=6); plug space bounding box `(9, 141, 36.1, 21, 172, 75)`; with `power.PARTS["J2"]` patched to x = 30.0 via `mock.patch.dict`, `jack_xz` x is -5.0.
  - New `test_plug_path_open_to_the_ceiling`: bracket and switch body do not overlap `plug_space()` (use `tray.overlaps`).
  - Replace `test_prints_top_down_with_the_feet_open_upwards` with `test_prints_on_its_rear_face`: for `p = arm_bracket_print()`, bounding-box height 28.0; solid at bed level (`b[2] + 0.15`) at the print positions of the two feet and the bar centre; bore open at both ends.

- [ ] **Step 2: Run tests, expect failures**

Run: `.venv/bin/python -m unittest test_tray`
Expected: FAIL in the tests above (missing `jack_xz`, `plug_space`, old heights).

- [ ] **Step 3: Implement in `tray.py`**
  - `jack_xz(t)`: x from `to_tray(t, *power.PARTS["J2"][3][:2])[0]`; z = `envelope(t)["power_board_bottom"] + t["board_t"] + t["jack_axis"]` (do not call `envelope` recursively from inside it if `envelope` uses it).
  - `plug_space(t)`: box x `jx ± plug_d/2`, y `stack2_y + stack.H` to `arm_span(t)[1] + 1`, z `jz - plug_d/2` to `ceiling`.
  - `arm_bar(t)`: y0 = `length + arm_bar_gap`, y1 = `arm_span(t)[1]`, z1 = `arm_bar_top`, z0 = z1 − `arm_bar_t`.
  - `switch_centre`: (`switch_x`, middle of the bar in y). `switch_space`: hangs `depth` below bar z0.
  - `envelope`: `switch_top = arm_bar_top + switch_above`, `switch_bottom = bar z0 − depth`. `check_envelope`: flag "arm switch body … floor" when `switch_bottom < min_floor_clear`.
  - `arm_bracket`: feet as now (blocks at |x| `cradle_inner`–`half_width`, y span, z `tz + arm_gap` to `arm_top`; the bore leaves the roof over the tube), legs box |x| `cradle_inner − arm_leg_w`–`cradle_inner`, bar y range, z bar z0 to `arm_top`; bar box |x| < `cradle_inner`, bar y and z. Cuts: bores, tie notches, switch hole through the bar.
  - `check_parts`: add "arm bracket: something is in the 12 V plug's path" when bracket overlaps `plug_space`, and include `plug_space` in the switch body's clash list.
  - `arm_bracket_print`: `rotate((-90.0, 0.0, 0.0))`, so the rear face is at the bottom.

- [ ] **Step 4: Run tests and the generator**

Run: `.venv/bin/python -m unittest test_tray && .venv/bin/python tray.py`
Expected: all OK; `arm-bracket: … cm3` printed with no problems.

- [ ] **Step 5: Commit**

```bash
git add hardware/turret-pdb/tools/tray.py hardware/turret-pdb/tools/test_tray.py
git commit -m "Lower the arm switch onto a cross-bar under the 12 V plug"
```

### Task 2: Docs

**Files:**
- Modify: `hardware/turret-pdb/README.md` (the `arm-bracket.stl` row, steps 6 and 8), `docs/wiring.md` (J3 row and the arm switch layout bullet)

- [ ] **Step 1: Update the text**: bracket now about 106 × 28 mm, printed rear face down; feet over the rear cradles, legs inboard, cross-bar behind the tray at about 30–34 mm; switch bushing up through the bar left of centre, mirroring the jack, nut on top; lever top about 56 mm; body about 12 mm above the floor; nothing over the 12 V jack, so its plug goes straight in from behind. Note that the bracket's size and positions come from `TRAY`.
- [ ] **Step 2: Check** that `grep -n "68 mm\|top face down\|in the middle" hardware/turret-pdb/README.md docs/wiring.md` finds no stale arm bracket text.
- [ ] **Step 3: Commit** with the spec and plan:

```bash
git add hardware/turret-pdb/README.md docs/wiring.md docs/superpowers/specs/2026-11-21-arm-bracket-low-crossbar-design.md docs/superpowers/plans/2026-11-21-arm-bracket-low-crossbar.md
git commit -m "Document the arm bracket's low cross-bar"
```
