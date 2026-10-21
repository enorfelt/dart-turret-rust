# Turret controller board (copper-tape prototype)

A two-layer KiCad 10 board for the turret controller's low-current side. It carries the ESP32-S3 in sockets, the three MP1584 bucks, the level shifter, the battery divider, the input diodes, a 12 V bench jack and the connectors to the servos, ESCs, sensors and arm switch. It is meant to be 3D printed with raised copper and covered in copper tape, not sent to a fab. The ESCs stay on the quadcopter PDB; this board takes one 20 AWG feed from a spare PDB pad pair.

Status: designed, ERC and DRC clean, not printed or assembled yet. Dimensions still to check before printing are under [Before printing](#before-printing).

100 × 95 mm, 4 × M3. The minimum track width is 2 mm, with 1 mm between tracks. The exceptions are pads and vias at 2.54 mm pitch, which need only 0.55 mm.

## Files

| Path | What |
|---|---|
| `turret-pdb.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_dru` | The KiCad project. Open `turret-pdb.kicad_pro` in KiCad 10 |
| `turret-pdb.kicad_sym`, `turret-pdb.pretty/` | Project symbols and the copper-tape footprints: 1.9 × 4 mm header pads, large ring pads, solder holes for wires |
| `tools/generate.py` | Writes all of the above from `tools/board.py` (placement and every track) and `tools/parts.py` (footprints, with the module dimensions in `DIM`). It also checks the netlist against `firmware/turret/src/board.rs` |
| `tools/route_gnd.py` | Maze router for the GND net on both layers. It prints `T(...)`/`V(...)` lines to paste into `board.py` after the other nets are placed; takes about 3 minutes |
| `tools/board_shapes.py` | Runs in KiCad's bundled Python and writes the exact board outline, drill holes and copper polygons of both layers to `out/shapes.json` |
| `tools/plates.py` | Builds the plates to print from `shapes.json`: raised-track plates, and channel-method bases and companions. Every STL is checked to be watertight before it is kept (needs `manifold3d`) |
| `tools/export.sh` | Runs ERC and DRC and writes `out/`: the print plates, a one-piece `preview.stl`, copper PDFs to use as a cutting guide, an assembly PDF and renders |

```sh
cd hardware/turret-pdb
python3 tools/generate.py   # only while the design lives in board.py, see below
tools/export.sh             # needs KiCad 10 on Windows; set KICAD_CLI to use another kicad-cli
```

For the print plates, `export.sh` uses `tools/.venv`:

```sh
python3 -m venv tools/.venv && tools/.venv/bin/pip install manifold3d numpy
```

`generate.py` overwrites the schematic and the board. Once you start editing in KiCad, stop running it, or carry your edits back into `board.py` first.

## Layout

- **F.Cu (front, component side)** carries most of GND as 3 mm tracks: a spine down the bucks' GND pins, a loop along the bottom edge, and branches to the connectors. It also carries VIN from the input diodes to the bucks, the 5 V distribution, and two short jumpers: 3V3 to the light gate resistors, and ARM_SENSE to the arm switch header.
- **B.Cu (back, solder side)** carries every other signal and supply, plus the GND branches that would otherwise have to cross a front track.
- **Every part is soldered on the back only.** Front copper never connects to a pad; it reaches the parts through 19 vias (7 GND, 5 for 5 V, 3 VIN, 2 for 3V3, 2 for ARM_SENSE), with a short back-side track from each via to its pad. A DRC rule ("Parts are soldered on the back only") flags any front track that touches a pad, including after edits in the KiCad GUI.
- Next to an ESP pin, a track runs either straight out or around through the gap between the two header rows. Two 2 mm tracks never enter neighbouring 2.54 mm pins from the same side.

## Before printing

| Check | Where | Value used |
|---|---|---|
| MP1584 hole spacing, IN+ to OUT+ and IN+ to IN−, measured on your modules | `DIM["mp1584_pad_dx"]`, `["mp1584_pad_dy"]` | 19.0 × 14.0 mm, **placeholder** |
| Distance between the two Freenove header rows | `DIM["esp_row_spacing"]` | 25.4 mm, scaled from Freenove's pinout drawing |
| Adafruit 757 row spacing | `DIM["shifter_row_spacing"]` | 10.16 mm, from Adafruit's EAGLE board |

If a value changes, run `generate.py`. The routes move with the pads, so run `export.sh` again and fix anything DRC reports in `board.py`.

The process is new, so print a small piece first. Cut the ESP header area out of the model in Bambu Studio, then tape and solder a few pins.

## Printing

`export.sh` writes each plate ready to print, flat side down and tracks up. The back plate is already mirrored. Both are 1.8 mm thick: a 1.2 mm base with 0.6 mm raised copper.

| File | Plate |
|---|---|
| `out/front-raised.stl` | Front (component side) |
| `out/back-raised.stl` | Back (solder side) |
| `out/preview.stl` | The whole board as one 3.6 mm solid, for checking that the two faces line up. Not for printing |

1. Import both plates into Bambu Studio and print them side by side. After printing they line up on the four M3 holes.
2. Optionally cut a small piece out with the Cut tool to try the process first.
3. Print in PETG or ASA. PLA softens under the iron. Use 0.2 mm layers, 4 walls, 100 % infill and ironing on the top surface, so the tape has a smooth surface to stick to.
4. Printed holes come out small. Drill them to size by hand: 1.2 mm for pins and leads, 1.0 mm for vias, 1.7 mm for the diodes and 1.6 mm for the wire holes.

## Copper tape

Tape each plate on its own, before joining them.

1. **Back plate (tracks).** Lay the tape over the tracks and press it onto the raised surface with a fingernail or a spudger. Cut along each track edge with a fresh blade, then peel the tape off the low areas. `out/back-copper-mirrored.pdf` shows the pattern as seen from the back.
2. **Front plate.** Do the same with `out/front-copper.pdf` as the guide. Both faces have only tracks and pads, so no wide areas need covering.
3. Where a track is wider than one strip, or two strips overlap at a bend, solder the seam. Copper tape adhesive does not conduct well.
4. Bolt the plates together with the M3 screws, with a thin layer of glue between them if you like, and open the holes through the tape.
5. Tin the 4 mm power tracks (VIN, PUSH_6V, +6V, +7V5) with a layer of solder. Tape is only 35–50 µm thick.

## Channel method (no knife trimming)

`export.sh` also writes a second set of parts for the method used by PCB Forge. The copper sits in 0.6 mm channels, and each channel is outlined by a narrow ridge. You sand the ridges, which cuts the tape around every trace, so no knife work is needed.

| File | Print | Use |
|---|---|---|
| `out/front-base.stl`, `out/back-base.stl` | Channels up | The two board plates: 1.8 mm thick, 0.6 mm deep channels where the copper goes, and a 0.6 mm wide, 0.4 mm tall ridge around each channel. The back one is already mirrored |
| `out/front-companion.stl`, `out/back-companion.stl` | Bosses up | Press plates. Their bosses are the copper shapes, mirrored and inset by 0.15 mm, so they drop 0.5 mm into the channels when the plate is laid face down. Pockets around the bosses clear the ridges |

1. Print the bases and companions in PETG or ASA, with no XY compensation.
2. Cover a base with copper tape. Lay its companion face down on top, line it up on the M3 holes, and press hard, or clamp it, so the tape is forced into every channel.
3. Remove the companion and sand only the ridges, with sandpaper on a flat block, until the plastic shows along every ridge. That cuts the tape around each trace; the tape in the channels stays.
4. The tape left on the flat areas between ridges is now isolated from the traces. Peel it off, or leave it if it is stuck down well; it connects to nothing.
5. Wipe off the dust with isopropyl alcohol, check with a meter that neighbouring tracks are isolated, then carry on from step 4 of [Copper tape](#copper-tape) and with [Assembly](#assembly) as for the raised-track version.

Seams still need soldering where two strips meet in a channel.

## Assembly

1. **Vias.** Push 0.8 mm bare copper wire through each via, solder it on both sides, and trim it flush.
2. **Parts.** Fit every part from the front and solder it on the back, like a single-sided board. That covers the resistors, capacitors, diodes, jack, headers and wires.
3. **MP1584 modules.** Mount them on ordinary male header pins with the plastic spacer in place, soldered on the back. Set each output before connecting anything, then lock the trimmer with a drop of glue. The MG996R is damaged above 7.2 V.
4. **ESP sockets.** Use two full 1×20 female strips. First pull the metal contacts out of every position the board has no hole for, marked x on the assembly PDF: grip each contact's tail with pliers and push it out. Then solder the remaining contacts on the back. The empty positions keep the strip in one piece without touching the tracks that run under it.
5. **Level shifter.** The Adafruit 757 mounts **upside down**: solder its two 6-pin headers on the component side of the breakout, so the parts face the board. The footprint is mirrored to match, and its A3/B3 and HV-side GND pins have bare holes only.

## Connectors

Pin 1 is the square pad.

| Ref | Function | Pins |
|---|---|---|
| J1 | Battery in (from a PDB pad pair, 20 AWG) | 1 VBAT, 2 GND, solder holes |
| J2 | 12 V bench adapter, 5.5 × 2.1 mm, centre positive | |
| J3 | Arm switch (DPST) | 1 6V switched (to pusher), 2 6V, 3 GND, 4 SENSE (GPIO 47). Pole A goes on 1–2 and pole B on 3–4. This still works with the plug reversed |
| J4 | Pusher servo | 1 GND, 2 6V switched, 3 signal (5 V) |
| J5, J6 | ESC left, right | 1 GND, 2 not connected (leave the BEC red wire free), 3 signal |
| J7 | Feetech TTLinker logic side | 1 5V, 2 GND, 3 TTLinker TXD, 4 TTLinker RXD. Swap 3 and 4 if `ping 0` gets no answer |
| J8 | TFmini Plus | 1 5V, 2 GND, 3 TFmini TX, 4 TFmini RX |
| J9 | Light gate | 1 IR LED anode (cathode to 3), 2 phototransistor collector, 3 GND (LED cathode and phototransistor emitter) |
| J10 | Vision node 5V | 1 5V, 2 GND, solder holes |
| J11 | Servo bus power to TTLinker VIN | 1 7.5V, 2 GND, solder holes |

## Differences from the loose wiring in `docs/wiring.md`

- **Level shifter channels:** A1/B1 is the pusher, A2/B2 the TTLinker TXD → GPIO 21, and A4/B4 GPIO 14 → TTLinker RXD. A3/B3 is unused. The firmware is unchanged.
- **Light gate LED:** it runs from 3V3 through 150 Ω (about 13 mA) instead of 5 V through 330 Ω, since there is no 5 V near the gate connector.
- **Divider and capacitors:** the battery divider sits on the board at the battery input, before D1. The 1000 µF sits at the pusher header.
- **Diode OR on the input:** two SB560 Schottky diodes feed the bucks from either the battery or the 12 V jack, so neither source can back-feed the other. The divider measures the battery only. On the jack alone it reads under 5 V ("no pack"), the same as on USB.
- **7.5 V rail:** the 7.5 V servo rail is an MP1584, rated 3 A peak. Two stalled SC15s draw 3.4 A, so a double stall can make it overheat and cut out.

## Bench mode (12 V jack)

- With only the jack plugged in, the bucks, ESP, gimbal and pusher have power, but the ESCs on the PDB do not.
- Arming works as it does on USB, so the pusher can be exercised without flywheels.
- A 12 V 2.5 A adapter covers normal servo tests, but not a stall of both SC15s plus the pusher.
- Unplug both the jack and the battery before plugging in USB, as with the XT60.
- After assembly, check that J1 reads about 0 V with only the jack plugged in. That shows nothing back-feeds the PDB.

## Safety checks after assembly

- With the arm switch open, there must be no continuity from +6V (J3 pin 2) to the pusher V+ (J4 pin 2). `PUSH_6V` only reaches J3 pin 1, J4 pin 2 and the 1000 µF; `generate.py` checks this.
- The ESC middle pins have no copper.
- Set and lock the buck outputs before the ESP, servos or sensors are plugged in.
