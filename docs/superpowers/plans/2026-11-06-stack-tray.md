# Stack Tray Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A script that generates a printable PLA tray and deck holding both electronics stacks under the turret's frame tubes, and checks them before writing STLs.

**Architecture:** One script, `hardware/turret-pdb/tools/tray.py`, holds every dimension in a `TRAY` dict. It works out the stack heights (`envelope`), checks them (`check_envelope`), builds the tray and deck with manifold3d in assembly coordinates, checks the parts against each other and against the tubes, the BMS and Stack 2 (`check_parts`), and writes STLs through `plates.write`. Stack 2's outline, holes and board gap come from `stack.py`.

**Tech Stack:** Python 3, manifold3d and numpy in `hardware/turret-pdb/tools/.venv`, stdlib `unittest`.

**Spec:** `docs/superpowers/specs/2026-11-06-stack-tray-design.md`

## Global Constraints

- Coordinates: x across the tubes (0 on the centre line, + towards the XT60 side), y rearwards from the blue base (0 at its rear face), z up from the floor. Parts are built in these assembly coordinates; `plates.write` shifts each STL to the origin.
- Everything prints in PLA, flat side down, without supports. The tray has no nut traps; screws go up from under the floor.
- Nothing above z = 75.
- Stack 2 dimensions come only from `stack.py` (`W`, `H`, `HOLES`, `GAP`); never copy them into `TRAY`.
- No code comments unless the code looks wrong but is not (CLAUDE.md).
- Run everything from `hardware/turret-pdb/tools` with `.venv/bin/python`. If `.venv` is missing: `python3 -m venv .venv && .venv/bin/pip install manifold3d numpy`.
- Test helper used throughout `test_tray.py`:

```python
def solid_at(m, x, y, z):
    return (m ^ Manifold.cube((0.2, 0.2, 0.2), center=True).translate((x, y, z))).volume() > 1e-6
```

## Review Focus

- Real tubes thicker than 12.0 mm (carbon tolerance) need a bigger bore: `check_envelope` must reject `bore_d - tube_d < 0.1` (test in Task 1), and the cradle test print exists to catch the rest.
- A PDB measuring 30.0 or 30.5 mm between holes must both take M3 screws through the pillars (test in Task 3).
- Thicker foam tape under the BMS (2 mm is common) raises it into the deck: `check_envelope` must flag it (test in Task 1).
- A change in `stack.py` (`W`, `GAP`, `HOLES`) must move the bosses and re-run the width and logic-board checks, not silently pass (tests in Tasks 1 and 2 read `stack` directly).
- A failed check must leave no STLs behind, so an old good-looking file is never printed from a bad design (test in Task 3).

---

### Task 1: Dimensions, heights and envelope checks

**Files:**
- Create: `hardware/turret-pdb/tools/tray.py`
- Test: `hardware/turret-pdb/tools/test_tray.py`

**Interfaces:**
- Produces: `TRAY: dict`, `envelope(t: dict = TRAY) -> dict[str, float]`, `check_envelope(t: dict = TRAY) -> list[str]` (one human-readable line per problem, empty when fine).

`TRAY` holds exactly these keys and values (mm):

| Key | Value | Key | Value |
|---|---|---|---|
| `ceiling` | 75.0 | `tube_d` | 12.0 |
| `tube_gap` | 72.0 | `tube_bottom` | 30.0 |
| `bore_d` | 12.2 | `tie_t` | 1.5 |
| `floor_bottom` | 20.0 | `floor_t` | 3.0 |
| `half_width` | 51.0 | `length` | 150.0 |
| `cradles_y` | (0.0, 55.0, 138.0) | `cradle_len` | 12.0 |
| `cradle_inner` | 33.0 | `tunnels_y` | (3.0, 9.0) |
| `tunnel` | (5.0, 2.0, 25.0) — y size, z size, z bottom | `stack2_y` | 68.0 |
| `boss_d` | 7.0 | `boss_h` | 4.0 |
| `hole_d` | 3.4 | `board_t` | 2.0 |
| `stack2_height` | 38.0 — power board underside to ESP top | `bms` | (61.0, 46.0) |
| `bms_y` | 31.0 | `bms_board_t` | 1.6 |
| `mosfet_h` | 3.35 | `foam_t` | 1.0 |
| `pocket_depth` | 1.0 | `pocket_margin` | 0.3 per side |
| `vents_y` | (21.0, 31.0, 41.0) | `vent` | (40.0, 4.0) — x, y |
| `posts` | ((-20, 3.5), (20, 3.5), (-20, 58.5), (20, 58.5)) | `post_d` | 7.0 |
| `deck_bottom` | 30.0 | `deck_t` | 3.0 |
| `deck_x` | 24.0 | `deck_y` | (0.0, 62.0) |
| `min_deck_gap` | 1.5 | `pdb_pattern` | 30.25 |
| `pdb_hole_d` | 3.6 | `pillar_d` | 8.0 |
| `pdb_bottom` | 47.0 | `pdb_t` | 1.6 |
| `xt60_h` | 19.0 | `min_tie_gap` | 1.0 |
| `nut_slot` | (5.7, 2.6, 3.0) — width, height, depth of its top below the pillar top | `tie_slot` | (2.5, 5.0) — short and long side |
| `tie_slots` | ((-4, 63, "y"), (4, 63, "y"), (-37, 27, "x"), (-37, 35, "x"), (37, 27, "x"), (37, 35, "x")) — centre and the axis of the long side | | |

`envelope(t)` returns: `tube_top = tube_bottom + tube_d`; `tie_top = tube_top + tie_t`; `power_board_bottom = floor_bottom + floor_t + boss_h`; `logic_bottom = power_board_bottom + board_t + stack.GAP`; `esp_top = power_board_bottom + stack2_height`; `bms_top = floor_bottom + floor_t - pocket_depth + foam_t + bms_board_t + mosfet_h`; `deck_bottom`; `pdb_bottom`; `xt60_top = pdb_bottom + pdb_t + xt60_h`.

`check_envelope(t)` reports, with these words in the message: `"ESP"`, `"XT60"` or `"zip ties"` when that top is above `ceiling`; `"zip ties"` when `pdb_bottom - tie_top < min_tie_gap`; `"MOSFETs"` when `deck_bottom - bms_top < min_deck_gap`; `"between the tubes"` when `stack.W >= tube_gap`; `"logic board"` when `logic_bottom <= tube_top`; `"bore"` when `bore_d - tube_d < 0.1`.

- [ ] **Step 1: Write the failing tests**

`test_tray.py` imports `os`, `tempfile`, `unittest`, `from manifold3d import Manifold`, `stack` and `tray`, defines `solid_at` from Global Constraints, and ends with `unittest.main()` under `__main__`.

```python
class Envelope(unittest.TestCase):
    def test_heights(self):
        e = tray.envelope()
        expected = {"tube_top": 42.0, "tie_top": 43.5, "power_board_bottom": 27.0, "logic_bottom": 46.0,
                    "esp_top": 65.0, "bms_top": 27.95, "deck_bottom": 30.0, "pdb_bottom": 47.0, "xt60_top": 67.6}
        for k, v in expected.items():
            self.assertAlmostEqual(e[k], v, places=6, msg=k)

    def test_default_passes(self):
        self.assertEqual(tray.check_envelope(), [])

    def assertFlags(self, word, **change):
        problems = tray.check_envelope(dict(tray.TRAY, **change))
        self.assertTrue(any(word in p for p in problems), problems)

    def test_esp_above_ceiling(self):        self.assertFlags("ESP", stack2_height=50.0)
    def test_xt60_above_ceiling(self):       self.assertFlags("XT60", pdb_bottom=56.0)
    def test_pdb_on_ties(self):              self.assertFlags("zip ties", pdb_bottom=44.0)
    def test_thick_foam_hits_deck(self):     self.assertFlags("MOSFETs", foam_t=2.0)
    def test_board_wider_than_gap(self):     self.assertFlags("between the tubes", tube_gap=stack.W)
    def test_logic_board_in_tubes(self):     self.assertFlags("logic board", floor_bottom=15.0)
    def test_bore_too_tight(self):           self.assertFlags("bore", tube_d=12.15)
```

- [ ] **Step 2: Run them and see them fail**

Run: `.venv/bin/python -m unittest test_tray -v`
Expected: ERROR, `No module named 'tray'`.

- [ ] **Step 3: Write `TRAY`, `envelope` and `check_envelope` in `tray.py`** as specified above. Import `stack` for `W` and `GAP`.

- [ ] **Step 4: Run the tests**

Run: `.venv/bin/python -m unittest test_tray -v`
Expected: 9 tests OK.

- [ ] **Step 5: Commit**

```bash
git add hardware/turret-pdb/tools/tray.py hardware/turret-pdb/tools/test_tray.py
git commit -m "Add the stack tray's dimensions and height checks"
```

### Task 2: The tray

**Files:**
- Modify: `hardware/turret-pdb/tools/tray.py`
- Test: `hardware/turret-pdb/tools/test_tray.py`

**Interfaces:**
- Consumes: `TRAY`, `envelope` (Task 1); `stack.W`, `stack.H`, `stack.HOLES`.
- Produces: `tubes(t=TRAY) -> Manifold`, `stack2_space(t=TRAY) -> Manifold`, `bms_space(t=TRAY) -> Manifold`, `tray(t=TRAY) -> Manifold`, `check_parts(t: dict, parts: dict[str, Manifold]) -> list[str]` for the keys `"tray"` (this task) and `"deck"` (Task 3 adds its checks).

Geometry (spec, "Tray"):
- `tubes`: two cylinders of `tube_d` along y from −10 to `length` + 10, centres at x = ±(`tube_gap`/2 + `tube_d`/2) = ±42, z = `tube_bottom` + `tube_d`/2 = 36.
- `stack2_space`: box x ±(`stack.W`/2 + 0.5), y `stack2_y` to `stack2_y` + `stack.H`, z `power_board_bottom` to `ceiling`.
- `bms_space`: box `bms` centred on (0, `bms_y`), z from the foam top to `bms_top`.
- `tray`: floor box; for each `cradles_y` and each side, a block x ±(`cradle_inner` to `half_width`), z floor top to the tube centre, minus a `bore_d` cylinder on the tube axis and the two tunnels (`tunnel`, at `tunnels_y` within the cradle, running the full block width in x); four bosses at `(hx - stack.W/2, hy + stack2_y)` for `(hx, hy)` in `stack.HOLES`; four `posts` up to `deck_bottom`; the pocket (`bms` + 2 × `pocket_margin`, `pocket_depth` deep); the vents; the tie slots; a `hole_d` hole through boss or post and floor at every boss and post. Use 96 circular segments for every cylinder.
- `check_parts` for `"tray"`: status `NoError` and `len(m.decompose()) == 1` (`"tray: not one watertight piece"`); `(tray ^ tubes).volume() > 1e-3` gives `"tubes"`; `(tray ^ stack2_space).volume() > 1e-3` gives `"Stack 2"`; `(tray ^ bms_space).volume() > 1e-3` gives `"BMS"`.

- [ ] **Step 1: Write the failing tests**

```python
class Tray(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = tray.tray()

    def test_one_watertight_piece(self):
        self.assertEqual(str(self.m.status()), "Error.NoError")
        self.assertEqual(len(self.m.decompose()), 1)

    def test_extent(self):
        for got, want in zip(self.m.bounding_box(), (-51, 0, 20, 51, 150, 36)):
            self.assertAlmostEqual(got, want, places=3)

    def test_parts_check_passes(self):
        self.assertEqual(tray.check_parts(tray.TRAY, {"tray": self.m}), [])

    def test_tight_bore_flagged(self):
        t = dict(tray.TRAY, bore_d=11.8)
        self.assertTrue(any("tubes" in p for p in tray.check_parts(t, {"tray": tray.tray(t)})))

    def test_cradles(self):
        for y0 in tray.TRAY["cradles_y"]:
            for x in (-42, 42):
                self.assertTrue(solid_at(self.m, x, y0 + 6, 29.5))
                self.assertFalse(solid_at(self.m, x, y0 + 6, 31))
                for ty in (3, 9):
                    self.assertFalse(solid_at(self.m, x, y0 + ty, 26))
                self.assertTrue(solid_at(self.m, x, y0 + 6, 26))

    def test_bosses_on_stack_holes(self):
        for hx, hy in stack.HOLES:
            x, y = hx - stack.W / 2, hy + 68
            self.assertTrue(solid_at(self.m, x + 2.6, y, 25))
            for z in (21.5, 25):
                self.assertFalse(solid_at(self.m, x, y, z))

    def test_posts(self):
        for x, y in tray.TRAY["posts"]:
            self.assertTrue(solid_at(self.m, x + 2.6, y, 26))
            for z in (21.5, 26):
                self.assertFalse(solid_at(self.m, x, y, z))

    def test_pocket_and_vents(self):
        self.assertFalse(solid_at(self.m, 0, 51, 22.5))
        self.assertTrue(solid_at(self.m, 0, 55, 22.5))
        for y in (21, 31, 41):
            self.assertFalse(solid_at(self.m, 0, y, 21.5))
        self.assertTrue(solid_at(self.m, 0, 26, 21.5))

    def test_tie_slots(self):
        for x, y, _ in tray.TRAY["tie_slots"]:
            self.assertFalse(solid_at(self.m, x, y, 21.5))
```

- [ ] **Step 2: Run them and see them fail**

Run: `.venv/bin/python -m unittest test_tray -v`
Expected: the `Tray` tests ERROR with `module 'tray' has no attribute 'tray'`; the `Envelope` tests still pass.

- [ ] **Step 3: Write `tubes`, `stack2_space`, `bms_space`, `tray` and the tray part of `check_parts` in `tray.py`**

- [ ] **Step 4: Run the tests**

Run: `.venv/bin/python -m unittest test_tray -v`
Expected: all tests OK.

- [ ] **Step 5: Commit**

```bash
git add hardware/turret-pdb/tools/tray.py hardware/turret-pdb/tools/test_tray.py
git commit -m "Build the stack tray with its tube cradles and Stack 2 bosses"
```

### Task 3: Deck, cradle test, writing the STLs

**Files:**
- Modify: `hardware/turret-pdb/tools/tray.py`, `hardware/turret-pdb/tools/export.sh`
- Test: `hardware/turret-pdb/tools/test_tray.py`

**Interfaces:**
- Consumes: everything from Tasks 1–2; `plates.write(path, m)` (writes a binary STL shifted to the origin and exits on non-manifold edges).
- Produces: `deck(t=TRAY) -> Manifold`, `cradle_test(t=TRAY) -> Manifold`, `main(out: str, t: dict = TRAY) -> None`, and `if __name__ == "__main__": main(sys.argv[1] if len(sys.argv) > 1 else "../out/tray")`.

Geometry (spec, "Deck"):
- `deck`: bar x ±`deck_x`, y `deck_y`, z `deck_bottom` to + `deck_t`, with a `hole_d` hole at each of `posts` and the three vents at `vents_y`; four pillars of `pillar_d` from the deck top to `pdb_bottom` at (±`pdb_pattern`/2, `bms_y` ± `pdb_pattern`/2), each with a `pdb_hole_d` hole down through the pillar and deck, and a `nut_slot` cut from the pillar axis outwards along ±x (the sign of the pillar's x), its top `nut_slot[2]` below the pillar top.
- `cradle_test`: `tray(t)` intersected with the box x 30–52, y `cradles_y[0]` to + `cradle_len`, z 19–40.
- `check_parts` for `"deck"`: one watertight piece; `(deck ^ tray).volume() > 1e-3` gives `"deck hits the tray"`; `(deck ^ bms_space).volume() > 1e-3` gives `"MOSFETs"`.
- `main`: collect `check_envelope(t)` and `check_parts(t, {"tray": ..., "deck": ...})`; if any, `sys.exit` with them one per line before creating anything. Otherwise `os.makedirs(out, exist_ok=True)`, write `tray.stl`, `deck.stl` and `cradle-test.stl` with `plates.write`, and print each part's volume in cm³.
- `export.sh`: run `"$PY" tools/tray.py out/tray` when no names are given or when `tray` is one of them, and add `tray` to `failed` if it fails. Never pass `tray` to `export_board`. Skip it with the same "Skipping" hint as the plates when manifold3d is missing.

- [ ] **Step 1: Write the failing tests**

```python
class Deck(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = tray.deck()

    def test_one_piece_up_to_the_pdb(self):
        self.assertEqual(len(self.m.decompose()), 1)
        self.assertAlmostEqual(self.m.bounding_box()[5], 47.0, places=3)

    def test_parts_check_passes(self):
        self.assertEqual(tray.check_parts(tray.TRAY, {"tray": tray.tray(), "deck": self.m}), [])

    def test_post_screws_go_through(self):
        both = tray.tray() + self.m
        for x, y in tray.TRAY["posts"]:
            rod = Manifold.cylinder(14, 1.5, 1.5, 32).translate((x, y, 20))
            self.assertLess((both ^ rod).volume(), 1e-6)

    def test_pdb_screws_fit_30_and_30_5(self):
        for pattern in (30.0, 30.5):
            for sx in (-1, 1):
                for sy in (-1, 1):
                    rod = Manifold.cylinder(14, 1.5, 1.5, 32).translate((sx * pattern / 2, 31 + sy * pattern / 2, 33))
                    self.assertLess((self.m ^ rod).volume(), 1e-6, (pattern, sx, sy))

    def test_nut_slots_open_outwards(self):
        for sx in (-1, 1):
            x = sx * 30.25 / 2
            self.assertFalse(solid_at(self.m, x + sx * 3.0, 31 + 15.125, 42.7))
            self.assertTrue(solid_at(self.m, x + sx * 3.0, 31 + 15.125, 39.0))


class Output(unittest.TestCase):
    def test_writes_three_stls(self):
        with tempfile.TemporaryDirectory() as d:
            tray.main(d)
            for name in ("tray.stl", "deck.stl", "cradle-test.stl"):
                self.assertGreater(os.path.getsize(os.path.join(d, name)), 1000)

    def test_failed_check_writes_nothing(self):
        with tempfile.TemporaryDirectory() as d:
            out = os.path.join(d, "tray")
            with self.assertRaises(SystemExit):
                tray.main(out, dict(tray.TRAY, foam_t=2.0))
            self.assertFalse(os.path.exists(out))

    def test_cradle_test_is_one_piece(self):
        self.assertEqual(len(tray.cradle_test().decompose()), 1)
```

- [ ] **Step 2: Run them and see them fail**

Run: `.venv/bin/python -m unittest test_tray -v`
Expected: `Deck` and `Output` tests ERROR with missing attributes `deck`, `main`, `cradle_test`.

- [ ] **Step 3: Write `deck`, `cradle_test`, the deck part of `check_parts`, `main` and the `__main__` block in `tray.py`; hook it into `export.sh`**

- [ ] **Step 4: Run the tests and the script**

Run: `.venv/bin/python -m unittest test_tray -v`
Expected: all tests OK.

Run: `.venv/bin/python tray.py ../out/tray && ls ../out/tray`
Expected: three volume lines, then `cradle-test.stl  deck.stl  tray.stl`.

- [ ] **Step 5: Commit**

```bash
git add hardware/turret-pdb/tools/tray.py hardware/turret-pdb/tools/test_tray.py hardware/turret-pdb/tools/export.sh
git commit -m "Add the stack tray's deck and write the tray STLs"
```

### Task 4: Documentation

**Files:**
- Modify: `hardware/turret-pdb/README.md`, `docs/wiring.md`, `CLAUDE.md`

- [ ] **Step 1: README.** In "Where it goes on the turret", correct the Stack 1 table: BMS 61 × 46 mm, about 5 mm thick, pads on the two 46 mm ends, held on foam tape in the tray; PDB 4 × M3 on 30–30.5 mm, XT60 about 19 mm tall pointing out over the +x tube. Add a "Stack tray" section after "Dimensions for the holders" covering: the three STLs and how to print them (PLA, flat side down, no supports, print `cradle-test.stl` first on a tube offcut); the hardware list from the spec's Documentation section; the assembly order from the spec; washers under every head and nut, screws only snug. Add `tray.py` and `test_tray.py` to the Files table and `.venv/bin/python -m unittest test_tray` (from `tools/`) to the commands block.

- [ ] **Step 2: wiring.md.** In "Where it sits", say both stacks sit in the printed stack tray under the tubes (link to the README section) and correct the BMS size to 61 × 46 mm. In the BOM, set the BMS size and add a line for the tray hardware (M3 screws 4 × 16, 4 × 12, 4 × 8 mm, 8 nuts, washers, 4.8 mm zip ties, 1 mm foam tape).

- [ ] **Step 3: CLAUDE.md.** Add `cd hardware/turret-pdb/tools && .venv/bin/python -m unittest test_tray && .venv/bin/python tray.py   # stack tray: tests, STLs in out/tray/` to the Commands block, and `tools/tray.py` (the stack tray around both stacks, dimensions in `TRAY`) to the `hardware/turret-pdb` line in Layout.

- [ ] **Step 4: Check.** Run `.venv/bin/python -m unittest test_tray` from `hardware/turret-pdb/tools`; expected OK. Read both doc diffs once for numbers that disagree with the spec.

- [ ] **Step 5: Commit**

```bash
git add hardware/turret-pdb/README.md docs/wiring.md CLAUDE.md
git commit -m "Document the stack tray"
```
