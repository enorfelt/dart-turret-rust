# Turret controller boards (3D-printed prototype)

Two stacked two-layer KiCad 10 boards for the turret controller's low-current side. They are meant to be 3D printed, with the tracks made of copper wire laid in grooves or of copper tape, but `export.sh` also writes Gerbers for [ordering them from a fab](#ordering-from-a-fab).

- **Power board** (bottom): battery input, 12 V bench jack, the two input diodes, the three MP1584 bucks, the battery divider, the arm switch and pusher connections with the 1000 µF, and the servo bus and vision 5 V outputs. All the high-current nets stay on this board.
- **Logic board** (top): the ESP32-S3 in sockets, the level shifter, the light gate resistors and the connectors to the ESCs, TTLinker, TFmini Plus and light gate.

Five wires join the two boards. The ESCs stay on the quadcopter PDB; the power board takes one 20 AWG feed from a spare PDB pad pair.

Status: designed, ERC and DRC clean on both boards, not printed or assembled yet. Dimensions still to check before printing are under [Before printing](#before-printing).

Both boards are 70 × 68 mm with 4 × M3 in the same places. The minimum track width is 2 mm, with 1 mm between tracks. The exceptions are pads and vias at 2.54 mm pitch, which need only 0.55 mm.

## Where it goes on the turret

The electronics sit between the two frame tubes (about 75 mm apart) behind the blue base that holds the batteries, in two stacks one behind the other. Nothing may rise above the top of the blue base, about 75 mm above the floor, because the turret sweeps that space.

```
        front (barrel)                                   rear
  ┌──────────────┐   ┌─────────────┐   ┌────────────────────┐
  │  blue base   │   │  Stack 1    │   │  Stack 2           │
  │  (battery)   │   │  PDB        │   │  logic board       │ ← ESP antenna at the rear edge
  │              │   │  BMS        │   │  power board       │
  └──────────────┘   └─────────────┘   └────────────────────┘
```

**Stack 1** holds bought boards:

| Layer | Part | Size | Mounting |
|---|---|---|---|
| Top | Quadcopter PDB with XT60 (Matek PDB-XT60 clone) | about 36 × 50 mm including the XT60 tab | 4 × M3 on 30.5 × 30.5 mm; measure your board before designing the holder |
| Bottom | 4S 40 A BMS | 55 × 44 mm | No holes; hold it by its edges |

**Stack 2** is these two boards on four M3 standoffs: the power board at the bottom, the logic board on top, with the ESP's antenna end pointing to the rear.

### Dimensions for the holders

Coordinates are in mm from the board's front-left corner, seen from the component side, with x across the tubes and y towards the rear. They come from `tools/stack.py`.

| What | Value |
|---|---|
| Outline | 70 (x, across the tubes) × 68 (y), 4 mm corner radius |
| M3 holes (3.2 mm) | (4, 4), (66, 4), (4, 64), (66, 64), the same on both boards |
| Board thickness | 2.0 mm (one wired plate, or two 1.0 mm taped plates) |
| Gap between the boards | 17 mm, from the top of the power board to the bottom of the logic board |
| Stack 2 height | about 41 mm: pin tails under the power board, both boards, the gap and the ESP in its sockets |
| Link pads | x 37.1, at y 33, 39, 45, 51, 57 on both boards |
| ESP USB connector | logic board, front end of the ESP at about (37, 11); its plug points forwards, about 12 mm above the board |
| ESP antenna | logic board, rear edge between x 23 and 51; keep metal away from it |

The gap is set by the tallest part on the power board (C2, a 6.3 × 11 mm can standing on its leads, about 12.2 mm) plus the pin tails on the logic board's back (about 3 mm) plus 1 mm; `generate.py` checks it. Every power-board connection is a solder hole, because a plug mated to a header would not fit under the logic board.

## Files

| Path | What |
|---|---|
| `power/`, `logic/` | One KiCad project per board: `turret-<board>.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_dru`, and its own symbol and footprint library. Open the `.kicad_pro` in KiCad 10 |
| `tools/stack.py` | What both boards share: outline, M3 holes, the board gap, and the link pads with their net order |
| `tools/power.py`, `tools/logic.py` | Each board's parts, placement and every track |
| `tools/parts.py` | Footprints, with the module dimensions in `DIM` |
| `tools/generate.py` | Writes both projects from the files above and checks them: the logic board's ESP pins against `firmware/turret/src/board.rs`, the arm switch interlock nets, the link pads lining up on both boards with the same nets, nothing under the ESP antenna, nothing in the USB plug's path and C3 lying flat |
| `tools/route.py` | Maze router for a board's nets on both layers. It prints `T(...)`/`V(...)` lines to paste into that board's module after the tracks already there |
| `tools/board_shapes.py` | Runs in KiCad's bundled Python and writes the exact board outline, drill holes, copper polygons and track centre lines of both layers to `shapes.json` |
| `tools/plates.py` | Builds the plates to print from `shapes.json`: the wired plate, raised-track plates, and channel-method bases and companions. Every STL is checked to be watertight before it is kept, and the wired plate's grooves are checked for wall thickness between nets (needs `manifold3d`) |
| `tools/export.sh` | Runs ERC and DRC and writes `out/<board>/` for each board: the print plates, a one-piece `preview.stl`, copper PDFs to use as a cutting guide, an assembly PDF, renders, and Gerbers and drill files for a fab in `gerbers/` and `turret-<board>-gerbers.zip` |

```sh
cd hardware/turret-pdb
python3 tools/generate.py           # both boards; only while the design lives in tools/, see below
tools/export.sh                     # both boards, or name one: tools/export.sh logic
python3 tools/route.py power GND    # route one or more nets of a board; GND alone takes under a minute
```

`export.sh` needs KiCad 10 on Windows; set `KICAD_CLI` to use another kicad-cli. For the print plates, it uses `tools/.venv`:

```sh
python3 -m venv tools/.venv && tools/.venv/bin/pip install manifold3d numpy
```

`generate.py` overwrites the schematics and the boards. Once you start editing in KiCad, stop running it, or carry your edits back into `tools/` first.

## Layout

On both boards every part is soldered on the back only. Front copper never connects to a pad; it reaches the parts through vias, with a short back-side track from each via to its pad. A DRC rule ("Parts are soldered on the back only") flags any front track that touches a pad, including after edits in the KiCad GUI.

**Power board.** The three bucks stand in a row across the front, outputs forwards, so each output is a short track to its connector on the front edge: J11 (7.5 V), J3 (arm switch, 6 V) and J10 (5 V). Wires to the arm switch (J3), the vision node (J10) and the pusher (J4) have no tie hole on the board; strain-relieve them on the holder. VIN runs as a 4 mm front-layer track under the modules from via to via, and the pusher's switched 6 V runs on the front from J3 down to J4 and C3, so signals and GND can cross both on the back. The diodes, the jack and the battery input sit at the rear right, the divider's 39 kΩ next to the battery input. 13 vias: 5 GND, 3 VIN, 3 PUSH_6V, 2 for 5 V.

**Logic board.** The ESP runs front to rear with its antenna at the rear edge and nothing under the antenna. The link pads and the level shifter sit between the two socket rows, under the Freenove board, which stands about 11 mm up on its sockets; the shifter is about 5.5 mm tall. Each shifter pin faces the ESP pin it connects to. The ESC, TTLinker and TFmini connectors are on the right; the light gate connector and its two resistors are on the left, next to GPIO 42. 5 V reaches the TFmini and the link pad on a front-layer track down the right side, and 3V3 runs on the front from the ESP's 3V3 pin to the shifter and the light gate. 14 vias: 5 GND, 3 for 3V3, 2 each for 5 V, PUSH_SIG and ARM_SENSE.

Next to an ESP pin, a track runs either straight out or into the gap between the two header rows. Two 2 mm tracks never enter neighbouring 2.54 mm pins from the same side.

## Before printing

| Check | Where | Value used |
|---|---|---|
| MP1584 hole spacing, IN+ to OUT+ and IN+ to IN−, measured on your modules | `DIM["mp1584_pad_dx"]`, `["mp1584_pad_dy"]` | 19.0 × 14.0 mm, measured |
| Distance between the two Freenove header rows | `DIM["esp_row_spacing"]` | 25.4 mm, measured |
| Adafruit 757 row spacing | `DIM["shifter_row_spacing"]` | 10.16 mm, from Adafruit's EAGLE board |
| Length of the 1000 µF body, lying on its side | `DIM["c3_length"]` | 20 mm (10 × 20 mm can) |
| Clear height under the Freenove board between its header rows, with it plugged into the sockets | — | at least 6 mm for the shifter, measured |

If a value changes, run `generate.py`. The routes move with the pads, so run `export.sh` again and fix anything DRC reports in `tools/power.py` or `tools/logic.py`.

The process is new, so print a small piece first. Cut the ESP header area out of the logic board's model in Bambu Studio, then tape and solder a few pins.

## Printing

`export.sh` writes three ways of building each board into `out/power/` and `out/logic/`: the [wire method](#wire-method), which is one plate with no tape, and two copper tape methods. The finished board is 2.0 mm thick whichever you choose, so 2.54 mm header pins and part leads still stick out about 1 mm on the back.

For the tape methods, every plate is ready to print flat side down and tracks up, and the back plates are already mirrored. Each plate is 1.0 mm thick: a 0.6 mm base with 0.4 mm raised copper.

| File (per board) | Plate |
|---|---|
| `front-raised.stl` | Front (component side) |
| `back-raised.stl` | Back (solder side) |
| `preview.stl` | The whole board as one 2.0 mm solid, for checking that the two faces line up. Not for printing |

1. Import both plates of a board into Bambu Studio and print them side by side. After printing they line up on the four M3 holes.
2. Optionally cut a small piece out with the Cut tool to try the process first.
3. Print in PETG or ASA. PLA softens under the iron. Use 0.2 mm layers, 4 walls, 100 % infill and ironing on the top surface, so the tape has a smooth surface to stick to.
4. Printed holes come out small. Drill them to size by hand: 1.2 mm for pins and leads, 1.0 mm for vias, 1.7 mm for the diodes and 1.6 mm for the wire holes.

## Copper tape

Tape each plate on its own, before joining them.

1. **Back plate (tracks).** Lay the tape over the tracks and press it onto the raised surface with a fingernail or a spudger. Cut along each track edge with a fresh blade, then peel the tape off the low areas. `back-copper-mirrored.pdf` shows the pattern as seen from the back.
2. **Front plate.** Do the same with `front-copper.pdf` as the guide. Both faces have only tracks and pads, so no wide areas need covering.
3. Where a track is wider than one strip, or two strips overlap at a bend, solder the seam. Copper tape adhesive does not conduct well.
4. Bolt the plates together with the M3 screws, with a thin layer of glue between them if you like, and open the holes through the tape.
5. Tin the power tracks on the power board (VIN, the battery input, the 12 V jack, 7.5 V, 6 V and the pusher's switched 6 V) with a layer of solder. Tape is only 35–50 µm thick.

## Channel method (no knife trimming)

`export.sh` also writes a second set of parts for the method used by PCB Forge. The copper sits in 0.4 mm channels, and each channel is outlined by a narrow ridge. You sand the ridges, which cuts the tape around every trace, so no knife work is needed.

| File (per board) | Print | Use |
|---|---|---|
| `front-base.stl`, `back-base.stl` | Channels up | The two board plates: 1.0 mm thick, 0.4 mm deep channels where the copper goes, and a 0.6 mm wide, 0.4 mm tall ridge around each channel. The back one is already mirrored |
| `front-companion.stl`, `back-companion.stl` | Bosses up | Press plates. Their bosses are the copper shapes, mirrored and inset by 0.15 mm, so they drop 0.3 mm into the channels when the plate is laid face down. Pockets around the bosses clear the ridges |

1. Print the bases and companions in PETG or ASA, with no XY compensation.
2. Cover a base with copper tape. Lay its companion face down on top, line it up on the M3 holes, and press hard, or clamp it, so the tape is forced into every channel.
3. Remove the companion and sand only the ridges, with sandpaper on a flat block, until the plastic shows along every ridge. That cuts the tape around each trace; the tape in the channels stays.
4. The tape left on the flat areas between ridges is now isolated from the traces. Peel it off, or leave it if it is stuck down well; it connects to nothing.
5. Wipe off the dust with isopropyl alcohol, check with a meter that neighbouring tracks are isolated, then carry on from step 4 of [Copper tape](#copper-tape) and with [Assembly](#assembly) as for the raised-track version.

Seams still need soldering where two strips meet in a channel.

## Wire method

The tracks are 0.75–0.8 mm tinned copper bus wire (about 20 AWG), pressed into grooves in a single printed plate. There is no tape, no knife work and no second plate to line up. The wire carries more current than any of the tape tracks, so the power tracks need no tinning.

`wired.stl` is the whole board as one 2.0 mm plate, already turned back face up for printing. Both faces have 1.0 mm wide, 0.6 mm deep grooves along the track centre lines. The grooves continue from where a track meets a pad into the pad's hole, and each hole has a solder well 0.4 mm wider than the hole. The back has wells at every hole, the front at the vias only, since no part is soldered on the front. `plates.py` stops if two nets' grooves, wells or holes come closer than 0.4 mm, and prints how much wire each face needs.

1. Print in PETG or ASA as exported, back face up, with 0.2 mm layers, 100 % infill, the Arachne wall generator and no ironing. The front grooves are on the bed side; they bridge over 1 mm and need no supports. Set elephant foot compensation to about 0.15 mm so they do not close up.
2. Drill the holes as for the tape plates. Vias need 1.0 mm for the wire.
3. **Front.** Straighten the wire by pulling it. Lay it into one groove at a time, bending it at the corners with pliers, and press it in with the iron at about 250 °C so it sinks into the plastic and stays. At a via, push the wire through the hole and leave a few millimetres on the back.
4. **Back.** Do the same. Bend each via tail from the front into the back groove that leaves that via, or solder it to that groove's wire in the well.
5. At a pad, stop the wire at the edge of the hole. The part's lead and the wire are soldered together in the well.
6. Solder every place where two wires meet. Check with a meter that neighbouring tracks are isolated, then carry on with [Assembly](#assembly) from step 2. The vias are already done.

## Ordering from a fab

`export.sh` writes each board's Gerbers (both copper layers, mask, silkscreen and outline, Protel file names, no X2) and Excellon drill files (mm, plated and unplated holes in separate files) into `out/<board>/gerbers/`, and zips them as `out/<board>/turret-<board>-gerbers.zip`. Upload the zip to JLCPCB or PCBWay as it is; both read these settings.

| Option | Value |
|---|---|
| Layers | 2 |
| Size | 70.1 × 68.1 mm, the same for both boards |
| Thickness | 1.6 mm is fine and cheapest; the board file says 2.0 mm only because that is the printed plate. Header pins still stick out about 1.4 mm on the back |
| Smallest hole | 1.0 mm plated; the M3 holes are 3.2 mm unplated |
| Copper | 1 oz is enough; the 5 V and VIN tracks are 2–4 mm wide |

On a fab board every pad and via is plated, so skip step 1 of [Assembly](#assembly). In step 7 the link wires can then be soldered on either face, since the J12 holes are plated too.

## Assembly

1. **Vias.** Push 0.8 mm bare copper wire through each via, solder it on both sides, and trim it flush. With the wire method this is already done.
2. **Parts.** Fit every part from the front and solder it on the back, like a single-sided board. That covers the resistors, capacitors, diodes, jack, headers and wires.
3. **MP1584 modules.** Mount them on ordinary male header pins with the plastic spacer in place, soldered on the back. Set each output before connecting anything, then lock the trimmer with a drop of glue. The MG996R is damaged above 7.2 V.
4. **1000 µF (C3).** Bend its leads 90° so it lies flat on the power board, body towards the board's left edge, as drawn on the assembly PDF. Standing up it would hit the logic board.
5. **Level shifter.** The Adafruit 757 mounts **upside down**: solder its two 6-pin headers on the component side of the breakout, so the parts face the board. The footprint is mirrored to match. Its A3/B3 and LV-side GND pins have bare holes only. Trim the header pins that stick up above the breakout, so it stays under 6 mm.
6. **ESP sockets.** Use two full 1×20 female strips. First pull the metal contacts out of every position the board has no hole for, marked x on the assembly PDF: grip each contact's tail with pliers and push it out. Then solder the remaining contacts on the back. The empty positions keep the strip in one piece without touching the tracks that run under it.
7. **Stack.** The link tracks run on the back of both boards and the holes are not plated, so each link wire must be soldered on the back face of both boards. Cut five wires about 35 mm long: 22 AWG for GND and 5V, 26 AWG for VBAT, ARM and PUSH. Push each into the logic board's J12 from the back and solder it on the back; the pad names are on the assembly PDF, seen from the front. Bolt the power board onto four M3 standoffs so its top face and the logic board's bottom face are 17 mm apart. Lower the logic board onto the standoffs, threading each wire down through the power board's J12 hole with the same name, and solder them on the power board's back. J12 sits between the ESP sockets, so do this before plugging in the ESP.

## Connectors

Pin 1 is the square pad.

**Power board**

| Ref | Function | Pins |
|---|---|---|
| J1 | Battery in (from a PDB pad pair, 20 AWG) | 1 VBAT, 2 GND, solder holes |
| J2 | 12 V bench adapter, 5.5 × 2.1 mm, centre positive | |
| J3 | Arm switch (DPST) | 1 6V switched (to pusher), 2 6V, 3 GND, 4 SENSE (GPIO 47), solder holes. Pole A goes on 1–2 and pole B on 3–4. This still works with the switch wired the other way round |
| J4 | Pusher servo | 1 GND, 2 6V switched, 3 signal (5 V), solder holes. Cut the servo plug off, or solder a short pigtail with a plug in line |
| J10 | Vision node 5V | 1 5V, 2 GND, solder holes |
| J11 | Servo bus power to TTLinker VIN | 1 7.5V, 2 GND, solder holes |
| J12 | Link to the logic board | 1 GND, 2 5V, 3 VBAT (divided), 4 ARM, 5 PUSH, solder holes |

**Logic board**

| Ref | Function | Pins |
|---|---|---|
| J5, J6 | ESC left, right | 1 GND, 2 not connected (leave the BEC red wire free), 3 signal |
| J7 | Feetech TTLinker logic side | 1 5V, 2 GND, 3 TTLinker TXD, 4 TTLinker RXD. Swap 3 and 4 if `ping 0` gets no answer |
| J8 | TFmini Plus | 1 5V, 2 GND, 3 TFmini TX, 4 TFmini RX |
| J9 | Light gate | 1 IR LED anode (cathode to 3), 2 phototransistor collector, 3 GND (LED cathode and phototransistor emitter) |
| J12 | Link to the power board | as on the power board |

## Differences from the loose wiring in `docs/wiring.md`

- **Level shifter channels:** A1/B1 is GPIO 14 → TTLinker RXD, A2/B2 the pusher, and A4/B4 TTLinker TXD → GPIO 21. A3/B3 is unused. The firmware is unchanged.
- **Light gate LED:** it runs from 3V3 through 150 Ω (about 13 mA) instead of 5 V through 330 Ω, since 3V3 is closer to the gate connector.
- **Divider and capacitors:** the battery divider sits on the power board at the battery input, before D1. Only the divided voltage goes up to the logic board. The 1000 µF sits at the pusher connector.
- **Diode OR on the input:** two SB560 Schottky diodes feed the bucks from either the battery or the 12 V jack, so neither source can back-feed the other. The divider measures the battery only. On the jack alone it reads under 5 V ("no pack"), the same as on USB.
- **7.5 V rail:** the 7.5 V servo rail is an MP1584, rated 3 A peak. Two stalled SC15s draw 3.4 A, so a double stall can make it overheat and cut out.

## Bench mode (12 V jack)

- With only the jack plugged in, the bucks, ESP, gimbal and pusher have power, but the ESCs on the PDB do not.
- Arming works as it does on USB, so the pusher can be exercised without flywheels.
- A 12 V 2.5 A adapter covers normal servo tests, but not a stall of both SC15s plus the pusher.
- Unplug both the jack and the battery before plugging in USB, as with the XT60.
- After assembly, check that J1 reads about 0 V with only the jack plugged in. That shows nothing back-feeds the PDB.

## Safety checks after assembly

- With the arm switch open, there must be no continuity from +6V (J3 pin 2) to the pusher V+ (J4 pin 2). `PUSH_6V` only reaches J3 pin 1, J4 pin 2 and the 1000 µF, all on the power board; `generate.py` checks this.
- The ESC middle pins have no copper.
- Check the five link wires with a meter through the joints, not on the J12 pads: GND from J10 pin 2 (power) to J7 pin 2 (logic), 5V from J10 pin 1 to J7 pin 1, and the other three from the power board part to the ESP socket contact (VBAT: R2 to GPIO 1, ARM: J3 pin 4 to GPIO 47, PUSH: J4 pin 3 to the shifter's B2). Do this before plugging in the ESP.
- Set and lock the buck outputs before the ESP, servos or sensors are plugged in.
