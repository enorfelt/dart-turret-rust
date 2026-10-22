# Turret Board Stack Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the 100 × 95 mm copper-tape controller board with a power board and a logic board that share one outline and stack on M3 standoffs, joined by five wires.

**Architecture:** The Python generator in `hardware/turret-pdb/tools` keeps its model (a module per board describes parts, placement and every track; `generate.py` writes the KiCad files and asserts the netlist), but builds two projects. Shared geometry and the interconnect live in `stack.py`. Checks stay as assertions in `generate.py` plus KiCad ERC/DRC run by `export.sh`.

**Tech Stack:** Python 3 (stdlib, numpy for `route_gnd.py`, manifold3d for plates), KiCad 10 `kicad-cli` on Windows called from WSL.

**Spec:** `docs/superpowers/specs/2026-10-22-turret-board-stack-design.md`

## Global Constraints

- Copper-tape rules unchanged: 2 mm minimum track, 1 mm clearance, 0.55 mm at 2.54 mm-pitch pads and vias, 1.2 mm base + 0.6 mm raised copper per plate, every part soldered on the back, front copper reaches pads only through vias.
- Outline: 70 mm across the tubes (x, hard limit 72) by about 62 mm along them (y); y = H is the rear edge. Identical on both boards.
- Board gap 15 mm; no power-board part taller than 11 mm.
- Interconnect: five wire pads at 6 mm pitch, nets in order GND, +5V, VBAT_SENSE, ARM_SENSE, PUSH_SIG, at the same XY on both boards.
- No change to GPIOs, nets or part values. `firmware/turret/src/board.rs` is not edited.
- No code comments unless the code looks wrong but is not (repo `CLAUDE.md`).
- Run everything from `hardware/turret-pdb`. `generate.py` overwrites the KiCad files; never hand-edit them.

## Review Focus

1. Interconnect wires crossed during assembly, because the logic board's pads are wired from below: each link pad carries its net name on the silkscreen of both boards (check in Task 2).
2. An M3 standoff or its nut lands on a part or track: `drc.json` for each board has no courtyard-overlap warning involving H1–H4 (Tasks 3 and 4).
3. The USB cable cannot reach the ESP: U1's USB end sits within 2 mm of the logic board's front edge, y = 0 (check in Task 2).
4. A power-board part is too tall for the gap: C3 uses the lying footprint, and the only standing radial part is C2 (6.3 × 11 mm) (check in Task 2).
5. KiCad opens a stale single-board project: no `.kicad_*` file, `.pretty` or lib table is left at the top of `hardware/turret-pdb` (Task 1).

---

### Task 1: Build the generator per board

Move the existing board unchanged into a `power` project so the refactor is proven by the old layout still passing ERC/DRC.

**Files:**
- Rename: `hardware/turret-pdb/tools/board.py` → `hardware/turret-pdb/tools/power.py` (content unchanged)
- Modify: `hardware/turret-pdb/tools/generate.py`, `hardware/turret-pdb/tools/route_gnd.py`, `hardware/turret-pdb/tools/export.sh`
- Delete: `hardware/turret-pdb/turret-pdb.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_dru`, `.kicad_sym`, `turret-pdb.pretty/`, `fp-lib-table`, `sym-lib-table`
- Generated: `hardware/turret-pdb/power/` (`turret-power.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_dru`, `.kicad_sym`, `turret-power.pretty/`, `fp-lib-table`, `sym-lib-table`)

**Interfaces:**
- Produces in `generate.py`:
  - `BOARDS = ("power", "logic")`
  - `ROOT`: absolute path of `hardware/turret-pdb`
  - `use(name: str) -> module`: imports the board module `name`, sets the module globals `board`, `NAME = f"turret-{name}"`, `LIB = NAME`, `PROJ = os.path.join(ROOT, name)`, `SHEET_UUID = uid("sheet", NAME)`, and returns the module. Every existing writer keeps reading these globals.
  - `main(names: list[str])`: runs the checks, then for each name `use(name)` and the five writers. CLI: `generate.py [power|logic]...`, no argument means every name in `BOARDS` whose module exists (only `power` in this task).
- Produces in `route_gnd.py`: CLI `route_gnd.py power|logic`. `START = {"power": "J1.2"}`. The grid globals (`NX`, `NY`, `X`, `Y`) are set in `init(name: str)` after `generate.use(name)`, not at import.
- Produces in `export.sh`: `tools/export.sh [power|logic]...`, default both existing boards; per board it reads `<b>/turret-<b>.kicad_*` and writes `out/<b>/`.

- [ ] **Step 1: Record the baseline**

Run: `python3 tools/generate.py && tools/export.sh`
Expected: `ERC 0, DRC errors 0, ... unconnected 0, schematic parity 0`. Keep a copy of `turret-pdb.kicad_pcb` in the scratchpad for Step 4.

- [ ] **Step 2: Rename `board.py` to `power.py` with `git mv`, add `use()` and `main(names)` to `generate.py`, make `route_gnd.py` and `export.sh` take the board name, delete the top-level KiCad files**

`verify()` keeps all its current assertions and runs against the `power` module for now. `write_footprints()` writes into `os.path.join(PROJ, f"{LIB}.pretty")` as before, which is now per board.

- [ ] **Step 3: Generate and export**

Run: `python3 tools/generate.py && tools/export.sh power`
Expected: `ERC 0, DRC errors 0, warnings 3, unconnected 0, schematic parity 0`, and `out/power/` holds the plates, PDFs and renders.

- [ ] **Step 4: Confirm the board is unchanged**

Run: `diff <(grep -v -E 'uuid|generator|sheetfile|turret-' <scratchpad copy>) <(grep -v -E 'uuid|generator|sheetfile|turret-' power/turret-power.kicad_pcb)`
Expected: no output.

- [ ] **Step 5: Confirm the router still runs**

Run: `python3 tools/route_gnd.py power > /dev/null`
Expected: exits 0 and prints no `# unreached` line on stderr.

- [ ] **Step 6: Confirm no stale top-level project (Review Focus 5)**

Run: `ls *.kicad_* turret-pdb.pretty fp-lib-table sym-lib-table 2>&1 | grep -v 'No such file'`
Expected: no output.

- [ ] **Step 7: Commit**

```bash
git add -A hardware/turret-pdb
git commit -m "Generate the controller board per board name, as the power project"
```

---

### Task 2: Split the netlist into two placed boards

Create `stack.py` and `logic.py`, move the logic parts out of `power.py`, place both boards inside the new outline, and add every new check. Tracks are not routed yet, so DRC reports unconnected items; ERC and the generator checks must pass.

**Files:**
- Create: `hardware/turret-pdb/tools/stack.py`, `hardware/turret-pdb/tools/logic.py`
- Modify: `hardware/turret-pdb/tools/power.py`, `hardware/turret-pdb/tools/generate.py`, `hardware/turret-pdb/tools/parts.py`, `hardware/turret-pdb/tools/route_gnd.py`
- Generated: `hardware/turret-pdb/logic/`

**Interfaces:**
- Consumes: `generate.use`, `generate.main` from Task 1.
- Produces in `stack.py`:
  - `W = 70.0`, `H = 62.0` (y grows towards the rear; may grow in Tasks 3–4 if routing needs it), `CORNER = 4.0`, `ORIGIN = (40.0, 40.0)`, `GAP = 15.0`
  - `HOLES`: four `(x, y)` M3 centres, 4 mm in from each corner
  - `LINK_NETS = ("GND", "+5V", "VBAT_SENSE", "ARM_SENSE", "PUSH_SIG")`, `LINK_NAMES = ("GND", "5V", "VBAT", "ARM", "PUSH")`
  - `LINK_AT = (x, y, rot)`: chosen in this task, within 6 mm of an edge, clear of the holes on both boards
  - `LINK_FP`: `parts.wire_pads(5)` with one `F.SilkS` text per pad giving its `LINK_NAMES` entry
  - `link() -> tuple`: the `PARTS` entry `("link", "Stack link", {"1": ("GND", "GND"), "2": ("5V", "+5V"), ...}, LINK_AT)`, used as `"J12"` on both boards
  - `holes() -> dict`: `{"H1": ("hole", "M3", {}, (x, y, 0)), ...}` from `HOLES`
- Produces in `power.py` and `logic.py`: `FP`, `PARTS`, `NETCLASSES`, `TRACKS`, `VIAS`, `T()`, `V()` as today. Both put `"link": stack.LINK_FP` and `"hole": parts.mounting_hole()` in `FP`, and `"J12": stack.link(), **stack.holes()` in `PARTS`. `W`, `H`, `CORNER` and `ORIGIN` leave the board modules.
- Produces in `parts.py`: `cap_radial_lying(dia: float, length: float, pitch: float) -> Footprint` named `CP_Radial_D{dia}mm_L{length}mm_P{pitch}mm_Lying`: the two pads as in `cap_radial`, and a `dia × length` body rectangle on F.Fab/F.SilkS/F.CrtYd starting 1.5 mm past the pads (the bent leads). `DIM["c3_length"] = 20.0`, listed in the README's "Before printing" table to be measured.
- Produces in `generate.py`: `outline()`, `edge()` and `placed()` read `stack.W`, `stack.H`, `stack.CORNER`, `stack.ORIGIN`. The checks become four functions, all run by `main` against both modules before anything is written:
  - `verify_firmware(logic)`: today's `board.rs` check, on `logic`
  - `verify_power(power)`: today's PUSH_6V, +6V, VJACK and VBAT member sets; C3's footprint is the lying capacitor; the only `cap_radial` part is C2
  - `verify_logic(logic)`: J5 and J6 have no pin 2; no track point (sampled every 0.5 mm along each segment, widened by half the track width) and no via lies in U1's antenna end, the footprint's local x range `[-esp_end_antenna, -1.27]` across its full y range; U1's USB end (local `x = 19 × 2.54 + esp_end_usb`) is within 2 mm of y = 0
  - `verify_stack(power, logic)`: both boards' `J12` and `H1`–`H4` entries equal `stack.link()` and `stack.holes()`; the nets present on both boards are exactly `set(LINK_NETS)`; every link net has at least one pad besides `J12` on each board; `LINK_FP` has one silk label per pad, matching `LINK_NAMES`
- Produces in `route_gnd.py`: `START = {"power": "J1.2", "logic": "J12.1"}`.

**Placement by board** (the spec's tables are the source; refs and values unchanged):

- `power.py` keeps J1, J2, D1, D2, C2, U3, U4, U5, R1, R2, C1, J11, J3, J4, C3 (now `cap_radial_lying(10.0, DIM["c3_length"], 5.0)`), J10, plus J12 and H1–H4.
- `logic.py` gets U1, U2, J5, J6, J7, J8, J9, R3, R4, plus J12 and H1–H4, and its own `ESP_USED`, `SHIFTER`, `esp_pins()`.
- U1 runs along y: antenna end at the rear edge (y near H), USB end at the front edge (y near 0).
- Both modules start with `TRACKS = []` and `VIAS = []`. Tracks from today's `board.py` are not carried over, since every part moves.

- [ ] **Step 1: Write `stack.py`, `parts.cap_radial_lying` and the four `verify_*` functions; make `main` load both `power` and `logic` modules**

With `logic.py` not yet written, `main` cannot load it.

- [ ] **Step 2: Run the checks and see them fail**

Run: `python3 tools/generate.py`
Expected: `ModuleNotFoundError: No module named 'logic'`.

- [ ] **Step 3: Split the parts into `power.py` and `logic.py` and place them**

Place both boards by editing coordinates, regenerating and looking at `out/<b>/render-top.png`. Leave room for 2 mm tracks with 1 mm gaps between parts, as in today's board. Pick `LINK_AT` where both boards have a clear edge.

- [ ] **Step 4: Run the checks and see them pass**

Run: `python3 tools/generate.py`
Expected: exits 0 and writes `power/` and `logic/`.

- [ ] **Step 5: Confirm each check can fail**

Temporarily break each in turn, run `python3 tools/generate.py`, expect an `AssertionError` naming it, then revert:
- (a) Swap pins 3 and 4 in `stack.link()`'s nets on the logic side only, by overriding `J12` in `logic.py`.
- (b) Add `T("GND", B, 2.0, <a point inside the antenna end>, <a nearby point>)` to `logic.py`.
- (c) Give C3 `cp10` again.
- (d) Add a `+3V3` pad to `power.py`.

- [ ] **Step 6: ERC and DRC without routing**

Run: `tools/export.sh`
Expected: for both boards `ERC 0`, `schematic parity 0`, `DRC errors 0`, `unconnected` non-zero. The script exits non-zero on the unconnected items; that is expected until Tasks 3–4.

- [ ] **Step 7: Commit**

```bash
git add -A hardware/turret-pdb
git commit -m "Split the controller board into placed power and logic boards"
```

---

### Task 3: Route the power board

**Files:**
- Modify: `hardware/turret-pdb/tools/power.py` (and `stack.py` only if `H` must grow, which then also needs `logic.py` placement re-checked)

**Interfaces:**
- Consumes: `T()`, `V()`, `stack.*` from Task 2; `route_gnd.py power` from Tasks 1–2.

- [ ] **Step 1: Route every net except GND in `power.py`**

Follow the netclass widths: VBAT, VJACK, VIN, +7V5, +6V and PUSH_6V at 3–4 mm, +5V at 3 mm, signals at 2 mm. PUSH_6V stays between J3.1, J4.2 and C3.1 only. Front copper reaches pads through vias only.

- [ ] **Step 2: Check before GND**

Run: `python3 tools/generate.py && tools/export.sh power`
Expected: `DRC errors 0`; the only unconnected items are GND.

- [ ] **Step 3: Route GND**

Run: `python3 tools/route_gnd.py power > "$TMPDIR/gnd.txt"` (about 3 minutes)
Expected: no `# unreached` lines on stderr. Paste the `T(...)`/`V(...)` lines into `power.py` after the other tracks.

- [ ] **Step 4: Full check**

Run: `python3 tools/generate.py && tools/export.sh power`
Expected: `ERC 0, DRC errors 0, ... unconnected 0, schematic parity 0`, and plates written without a non-manifold error.

- [ ] **Step 5: Standoff clearance (Review Focus 2)**

Run: `python3 -c "import json; d=json.load(open('out/power/drc.json')); print([v['description'] for v in d['violations'] if v['type']=='courtyards_overlap' and any(i['description'].startswith('Footprint H') for i in v['items'])])"`
Expected: `[]`

- [ ] **Step 6: Look at `out/power/render-top.png`**

Expected: J12 at `LINK_AT`, C3 lying flat, no part outside the outline.

- [ ] **Step 7: Commit**

```bash
git add hardware/turret-pdb
git commit -m "Route the power board"
```

---

### Task 4: Route the logic board

**Files:**
- Modify: `hardware/turret-pdb/tools/logic.py`

**Interfaces:**
- Consumes: as Task 3, with `route_gnd.py logic`.

Same steps as Task 3 with `logic` in place of `power`, plus:

- [ ] **Step 1: Route every net except GND in `logic.py`**

Next to an ESP pin, a track runs straight out or through the gap between the header rows, and two 2 mm tracks never enter neighbouring pins from the same side, as on today's board. Nothing runs under the antenna end; `verify_logic` enforces it.

- [ ] **Step 2: Check before GND**

Run: `python3 tools/generate.py && tools/export.sh logic`
Expected: `DRC errors 0`; the only unconnected items are GND.

- [ ] **Step 3: Route GND**

Run: `python3 tools/route_gnd.py logic > "$TMPDIR/gnd.txt"`
Expected: no `# unreached` lines. Paste the output into `logic.py`.

- [ ] **Step 4: Full check of both boards**

Run: `python3 tools/generate.py && tools/export.sh`
Expected: both boards `ERC 0, DRC errors 0, ... unconnected 0, schematic parity 0`, and plates written for both.

- [ ] **Step 5: Standoff clearance (Review Focus 2)**

The Task 3 Step 5 command on `out/logic/drc.json`.
Expected: `[]`

- [ ] **Step 6: Look at both renders side by side**

Expected: J12 and H1–H4 in the same place on both; the antenna end at the rear edge with no copper under it; the USB end at the front edge.

- [ ] **Step 7: Commit**

```bash
git add hardware/turret-pdb
git commit -m "Route the logic board"
```

---

### Task 5: Documentation

**Files:**
- Modify: `hardware/turret-pdb/README.md`, `docs/wiring.md`, `docs/wiring.html`, `CLAUDE.md`

**Interfaces:**
- Consumes: final `stack.W`, `stack.H`, `stack.HOLES`, `stack.LINK_AT`, `stack.GAP` from Tasks 2–4; the README must quote the values in the committed `stack.py`.

- [ ] **Step 1: Rewrite `hardware/turret-pdb/README.md`**

It needs:
- The two boards and Stack 2.
- A holder-dimension table: outline, hole centres, link pad centres, the 15 mm gap, Stack 2 height about 42 mm.
- Stack 1 part sizes: PDB about 36 × 50 mm on 30.5 × 30.5 mm M3, to be measured; BMS 55 × 44 mm with no holes.
- The files table with `stack.py`, `power.py` and `logic.py`, and the per-board commands.
- Printing and copper tape per board, with `out/<b>/` paths.
- Assembly order: power board, standoffs, the five link wires (22 AWG GND and +5V, 26 AWG the rest), then the logic board.
- C3 mounted lying, with `c3_length` added to "Before printing".
- The connectors table split by board, with J12.

The channel method, bench mode and safety sections stay, with paths updated.

- [ ] **Step 2: Update `docs/wiring.md`**

- **Controller board section:** describe the stack.
- **Battery section:** the BMS moves to Stack 1. The pack leads (14 AWG) and the B1–B3 taps (26 AWG) run from the battery covers in the blue base to it, with strain relief where they leave the base.
- **Power drawing and notes:** BMS → fuse → XT60 → PDB is the loop out of Stack 1, with the XT60 reachable.
- **PDB pads table:** pad pair 3 feeds the power board (20 AWG), and pad pairs 4–5 are spare when the board is used.

- [ ] **Step 3: Mirror the same changes in `docs/wiring.html`**

The battery pack wire table rows (around line 503) and the PDB pads table (around line 419), and the meta note at line 225.

- [ ] **Step 4: Update `CLAUDE.md`**

In the `hardware/turret-pdb` line, `tools/board.py` becomes `tools/power.py` and `tools/logic.py` with shared outline and interconnect in `tools/stack.py`; GND output is pasted into the board's own module. The board command stays `python3 tools/generate.py && tools/export.sh`, with "both boards" in its comment.

- [ ] **Step 5: Check the numbers**

Run: `grep -n -E "100 × 95|board\.py|turret-pdb\.kicad" hardware/turret-pdb/README.md docs/wiring.md docs/wiring.html CLAUDE.md`
Expected: no output. Then compare each dimension in the README table against `tools/stack.py` by eye.

- [ ] **Step 6: Commit**

```bash
git add hardware/turret-pdb/README.md docs/wiring.md docs/wiring.html CLAUDE.md
git commit -m "Document the board stack and where the PDB and BMS go"
```
