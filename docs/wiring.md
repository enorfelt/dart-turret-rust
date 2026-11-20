# Wiring

Turret controller wiring for a Freenove ESP32-S3 WROOM board (ESP32-S3-WROOM-1, octal PSRAM) on a 4S1P 18650 pack. Based on the [original Dart-Turret wiring](https://github.com/Tops76/Dart-Turret), with a main fuse, a power switch, a hardware arm switch, a separate pusher supply and pins moved for the Freenove board. Everything on the low-current side is on two stacked [controller boards](#controller-boards) from `hardware/turret-pdb`; the flywheel ESCs stay on a quadcopter power distribution board (PDB).

Status: controller boards made by JLCPCB and assembled; the turret is not fully wired and tested yet. `firmware/turret/src/board.rs` uses this pin map, and `hardware/turret-pdb/tools/generate.py` checks the logic board against it.

The same wiring drawn as colour-coded schematics is in [wiring.html](wiring.html); open it in a browser. Keep the two in step when the wiring changes.

## Pin map (Freenove ESP32-S3 WROOM)

The original pins do not fit this board: GPIO 33–37 are taken by the octal PSRAM (33/34 are not even broken out), and GPIO 38–40 go to the SD card slot. The camera connector stays empty on the turret controller, so camera pins are free to use.

| Function | GPIO | Was | Notes |
|---|---|---|---|
| Servo bus TX (UART1) | 14 | 33 | Level shifter A1/B1, then logic board J7 pin 4 to TTLinker RXD |
| Servo bus RX (UART1) | 21 | 34 | TTLinker TXD on J7 pin 3, then level shifter B4/A4 |
| ESC left | 17 | 17 | Logic board J5. Camera pin, unused here |
| ESC right | 18 | 18 | Logic board J6. Camera pin, unused here |
| Pusher servo | 41 | 39 | Level shifter A2/B2, link PUSH, power board J4 pin 3. JTAG MTDI, free while debugging over USB |
| Light gate | 42 | 38 | Logic board J9 pin 2. JTAG MTMS, free while debugging over USB |
| Battery divider | 1 | 16 | ADC1_CH0, so readings keep working with Wi-Fi / ESP-NOW on. Divider on the power board, link VBAT |
| TFmini Plus RX (ESP receives) | 6 | – | Logic board J8 pin 3. UART2, step 3 |
| TFmini Plus TX (ESP sends) | 7 | – | Logic board J8 pin 4. UART2, step 3 |
| Arm switch sense | 47 | – | Internal pull-up; switch closes to GND when armed. Power board J3 pin 4, link ARM |

Avoid: 0, 3, 45, 46 (strapping), 2 (LED), 48 (WS2812), 19/20 (USB), 43/44 (UART0 to the onboard USB-UART), 35–37 (PSRAM), 38–40 (SD card).

## Power

```
 4S1P 18650 pack (+ 4S BMS)
   +  ──[ 25 A blade fuse ]──[ XT60 + ]───────┬──────────── + bus (PDB)
   −  ──────────────────────[ XT60 − ]────────┼──────────── − bus (PDB)
                                              │
     PDB pad pairs (keep short):              │
       1 ── ESC left   (16 AWG) ─── motor left   (3 phase)
       2 ── ESC right  (16 AWG) ─── motor right  (3 phase)
       3 ── power board J1 (20 AWG)

 Power board
   J1 VBAT ──┬── R1 39 kΩ ──┬── R2 8.2 kΩ ── GND
             │              ├── C1 100 nF ── GND
             │              └── link VBAT ── logic board ── GPIO 1
             └── D1 SB560 ──┬── VIN ── C2 100 µF ── GND
   J2 12 V jack ── D2 SB560 ┘
   VIN ── U3 MP1584 7.5 V ── J11 ── TTLinker VIN ── SC15 pan ── SC15 tilt
       ── U4 MP1584 6 V ──── J3 2 ── arm switch pole A ── J3 1 ──┬── J4 2 ── pusher servo +
       │                                                         └── C3 1000 µF ── GND
       └─ U5 MP1584 5 V ──┬── J10 ── vision board 5V pin (through the gimbal)
                          └── link 5V ── logic board: ESP 5V pin, level shifter HV,
                                         J7 TTLinker 5 V, J8 TFmini 5 V
```

- The XT60 is the power switch: unplug it to turn the turret off. At 4S the spark from the ESC input capacitors is small; plug in quickly and firmly, without wiggling.
- The ESC capacitor inrush on plug-in can trip the BMS short-circuit protection. Plug in a few times on the bench. If the turret stays dark, the BMS has latched off: plug the charger into the pack for a moment to unlock it (see [Charging](#charging)). If it trips often, use an anti-spark XT60 with a pre-charge resistor.
- The fuse goes as close to the battery + as possible, on the pack side of the XT60, so it protects everything downstream.
- The + and − buses are the quadcopter PDB. Every + pad on it is the same copper, and so is every − pad, so each lead in the PDB pads table below can go on any pad of the right polarity. Soldered copper adds well under 1 mΩ and does not loosen from flywheel vibration.
- Two SB560 Schottky diodes OR the battery (D1) and the 12 V bench jack (D2) into the bucks, so neither source back-feeds the other and the ESCs on the PDB stay unpowered on the jack. See [Bench mode](#bench-mode-12-v-jack).
- The battery divider sits at J1, before D1, so it measures the pack only. Only the divided voltage goes up the link to the logic board, and C1 filters noise on it. On the jack alone it reads under 5 V, as "no pack", the same as on USB.
- The divider is on the turret side of the XT60, so its 0.36 mA does not drain the pack while unplugged.
- All grounds are common: the PDB − bus, the power board, the logic board, ESC signal grounds, TTLinker, TFmini and vision board.
- Only signal and ground from the ESC servo leads reach the logic board. The middle pin of J5 and J6 has no copper, so an ESC's BEC red wire stays unconnected.
- The SC15 is rated 4.8–8.4 V and stalls at 1.7 A, so two stalled servos need 3.4 A. U3 is an MP1584, rated 3 A peak, so it may overheat and cut out if both stall.
- The pusher servo (MG996R class) stalls at about 2.5 A and draws spikes of that size every time it starts to move. C3, a 1000 µF 16 V low-ESR electrolytic at J4, supplies those spikes, so they do not sag the 6 V rail or bounce the shared ground. U4 handles normal pushes with C3, but overheats and cuts out during a sustained stall.
- With pole A open, C3 holds enough charge to run the servo for a few milliseconds, far too short to push a dart, so the arm switch still cuts the pusher.
- Set each buck before anything is connected, then lock its trimmer with a drop of glue. Above 7.2 V the MG996R is damaged.
- Unplug the XT60 and the 12 V jack before plugging in USB, unless the Freenove board's schematic shows that VBUS and the 5V pin are isolated by a diode.

### PDB pads

| PDB pad | Connect |
|---|---|
| Battery input (the two large holes on the tab) | Male XT60 pigtail, 14 AWG; the board XT60 is left off |
| Pad pair 1 | ESC left, 16 AWG |
| Pad pair 2 | ESC right, 16 AWG |
| Pad pair 3 | Power board J1, 20 AWG |
| Pad pairs 4–6 | Spare |
| Small VCC / GND pads | Empty: the divider is on the power board |
| 5 V / 12 V BEC pads | Empty: the PDB's own regulators are not used |

### Bench mode (12 V jack)

- With only the jack plugged in, the bucks, ESP, gimbal and pusher have power, but the ESCs on the PDB do not.
- Arming works as it does on USB, so the pusher can be exercised without flywheels.
- A 12 V 2.5 A adapter (5.5 × 2.1 mm, centre positive) covers normal servo tests, but not a stall of both SC15s plus the pusher.
- With only the jack plugged in, J1 should read about 0 V. That shows nothing back-feeds the PDB.

## Controller boards

`hardware/turret-pdb` holds two stacked two-layer KiCad 10 boards, 70 × 68 mm, made by JLCPCB at 1.6 mm. The [board README](../hardware/turret-pdb/README.md) has the assembly steps, the stack tray and the safety checks.

- **Power board** (bottom): battery input, 12 V bench jack, the two input diodes, the three MP1584 bucks hanging on its back, the battery divider, the arm switch and pusher connections with C3, and the servo bus and vision 5 V outputs. Every connection is a solder hole, as a plug would not fit under the logic board.
- **Logic board** (top): the ESP32-S3 in sockets, the Adafruit 757 level shifter upside down between the socket rows, the light gate resistors and 2.54 mm headers for the ESCs, TTLinker, TFmini Plus and light gate.
- Five wires about 35 mm long join the boards through J12 on both: 22 AWG for GND and 5V, 26 AWG for VBAT, ARM and PUSH.

Pin 1 is the square pad.

**Power board** (soldered wires)

| Ref | Goes to | Pins | AWG |
|---|---|---|---|
| J1 | PDB pad pair 3 | 1 VBAT, 2 GND | 20 |
| J2 | 12 V bench adapter, 5.5 × 2.1 mm, centre positive | Barrel jack | – |
| J3 | Arm switch (DPST), on the bracket behind the tray | 1 and 2 to pole A (1 switched 6 V to the pusher, 2 6 V from U4), 3 and 4 to pole B (3 GND, 4 sense to GPIO 47). Either pole can be A | 20 for 1–2, 26 for 3–4 |
| J4 | Pusher servo | 1 GND (brown), 2 switched 6 V (red), 3 signal at 5 V (orange). Cut the servo plug off, or solder a short pigtail with a plug in line | Servo lead |
| J10 | Vision board 5V pin and GND, through the gimbal | 1 5V, 2 GND | 22 |
| J11 | TTLinker VIN and GND | 1 7.5 V, 2 GND | 20 |
| J12 | Link to the logic board | 1 GND, 2 5V, 3 VBAT (divided), 4 ARM, 5 PUSH | 22 / 26 |

**Logic board** (male headers)

| Ref | Goes to | Pins | AWG |
|---|---|---|---|
| J5, J6 | ESC left, right servo lead | 1 GND, 2 not connected (BEC red wire), 3 signal | Servo lead |
| J7 | TTLinker logic side | 1 5V, 2 GND, 3 TTLinker TXD, 4 TTLinker RXD. Swap 3 and 4 if `ping 0` gets no answer | 26 |
| J8 | TFmini Plus | 1 5V (red), 2 GND (black), 3 TFmini TX, 4 TFmini RX | 26 |
| J9 | Light gate | 1 IR LED anode, 2 phototransistor collector, 3 GND (LED cathode and phototransistor emitter) | 26 |
| J12 | Link to the power board | as on the power board | 22 / 26 |

### Where it sits

The electronics go between the two frame tubes behind the blue base, in two stacks one behind the other, and stay below the top of the blue base, which the turret sweeps.

- **Stack 1, behind the base:** the 4S BMS at the bottom on foam tape (61 × 46 mm, no holes), and the PDB on top on its 30–30.5 mm M3 pattern (about 36 × 43 mm with the battery tab). The PDB's board XT60 is left off, as the battery plug on top of the upright connector would reach into the turret's sweep. Instead a male XT60 pigtail on the tab's pads meets the female pigtail from the fuse and the BMS P− just outside the right-hand tube, each tied to the tube, so the pair stays within reach as the power switch. The balance plug from the BMS is tied next to the female pigtail, so the pack charges there without taking the cells out.
- **Stack 2, behind Stack 1:** the power board at the bottom on four printed risers, buck modules down, and the logic board on top, 17 mm above it on M3 standoffs, about 41 mm tall in all, with the ESP antenna at the rear edge. The two boards are joined on the tray: the link wires stand up from the power board and are soldered on the logic board's front. Outline, hole and link pad positions for the holders are in the board README.
- Both stacks sit in one printed [stack tray](../hardware/turret-pdb/README.md#stack-tray) that hangs under the tubes on zip ties.
- The arm switch, a mini toggle, stands lever up in the middle of a printed bracket that bridges the tubes over the tray's rear cradles, so it is reachable from behind. Its J3 leads run back along the side of Stack 2.

## Battery pack

The printed holder takes the four cells as two stacks of two, lying front to back. The covers hold contact plates, so the plates make the series links and no wire runs between cells. The cells charge in place, through the pack-side XT60 and a balance lead from the BMS (see [Charging](#charging)), so they only come out to be replaced.

Back cover (seen from behind), front cover (seen from the front):

```
 BACK                                      FRONT (left and right swap)
          left stack     right stack                 right stack    left stack
 top:     PACK − (S)     PACK + (B)         top:      (S) ┐          ┌ (B)
 bottom:  (B) ══ back bridge ══ (S)                       ║ bridge   ║ bridge
                                            bottom:   (B) ┘          └ (S)
 (S) spring, on a cell's − end    (B) flat button, on a cell's + end
```

Current path: pack − → top-left cell → front-left bridge → bottom-left cell → back bridge → bottom-right cell → front-right bridge → top-right cell → pack +. The two cells in a stack point opposite ways, and so do the two cells in a row. Every cell has a spring on its − end and a button on its + end.

The drawing assumes the long channel in the back cover sits over the bottom row. If it sits over the top row, swap top and bottom in the drawing; the rest stays the same.

| Wire | From | To | AWG |
|---|---|---|---|
| Pack − | Back end plate, top-left | BMS B− | 14 |
| Pack + | Back end plate, top-right | BMS B+, then the fuse and XT60 + | 14 |
| B1 | Front-left bridge | BMS B1 | 26 |
| B2 | Back bridge | BMS B2 | 26 |
| B3 | Front-right bridge | BMS B3 | 26 |
| Back bridge link | Flat plate on C2 | Spring plate on C3 | 14, or nickel strip ≥8 mm wide and 0.3 mm thick (two layers of 0.15 mm) |
| Output − | BMS P− | XT60 −, then the − bus | 14 |
| Balance lead | BMS B−, B1, B2, B3, B+ | JST-XH 5-pin plug, pins 1–5 in that order | 22–26 |

- The BMS sits in Stack 1 behind the blue base (see [Where it sits](#where-it-sits)), so the pack leads and B1–B3 run from the battery covers out of the base to it. Strain-relieve them where they leave the base and again at the BMS, and keep them clear of the pan axis.
- Pack leads and B2 leave through the hole in the back cover. The front cover has no hole: drill a 3 mm hole or file a notch for B1 and B3, and run them back along the frame.
- The back bridge link carries the full pack current, like the pack leads. Lay it in the long channel of the back cover.
- Solder to the plate tabs before the plates go into the covers, with the cells out. Nickel-plated steel takes solder with a hot iron and flux.
- Let the cover slots locate the plates; test-fit a plate before gluing. Then fix them with 2-part epoxy on the edges and tabs only, never on the contact face or the spring coils. Avoid hot glue, which softens near 60 °C, and superglue, which cracks with repeated cell swaps. Heat-staking the plate over a printed pin also works.
- Strain-relieve every wire on the cover a few cm from its solder joint, with a zip tie through a printed loop or epoxy over the wire. Pulled wires are what tear plates loose.
- The holder's pockets are 19.2 mm across at a 19.0 mm pitch within a stack, and its body is 65.0 mm long. The front cover's plate recess is 10.1 × 30.5 mm and 2.1 mm deep. With 65.2 mm cells, 0.3 mm plates and a back recess as deep as the front one, that leaves about 3.4 mm per cell for one button dome plus one compressed spring. Check the dome height plus the spring's solid height against that before gluing; if they do not fit, the cover cannot close.
- Keep solder joints out from under the cells, as a blob under a dome or spring lifts the plate in its 2.1 mm recess.
- After assembly, every spring should be visibly compressed, the covers should close without bulging, and the pack should read 12.8–16.8 V.
- If the springs bottom out and the front cover only closes under hard pressure, print a shim from `hardware/turret-pdb/tools/battery_shim.py` (1.0, 1.6 or 2.2 mm, in PETG or ASA) and fit it between the holder and the cover. A 2 mm slit in each side lets B1 and B3 slide into the cell openings, so the plates stay in the cover. Pick the thinnest one that lets the cover seat by hand while every spring stays compressed. The original M3×12 screws still reach about 7 mm into the holder with the thickest one.
- Mark + and − next to every pocket. A reversed cell makes the pack read about 8 V; the firmware then refuses `arm` and reports a battery fault.
- Use unprotected flat-top or button-top high-drain cells. Protected cells are longer and limit the current far below the spin-up peak.
- The 8 contacts in the chain are the weak point: at a 35 A spin-up peak, about 10 mΩ each sags the pack by almost 3 V. Use thick plates, print the covers in PETG or ASA rather than PLA, and measure the drop across a contact during a spin-up; more than about 0.1 V means that contact needs attention.
- The springs are the worst of them. 18650 spring plates are sized for the cell, not for its current: they are made for torches and power banks drawing a few amps. Until a spring is fully compressed, the current runs along the whole coil, about 10 cm of thin steel wire, so one spring can be 30–60 mΩ. Four in series sag the pack by 4–7 V at the spin-up peak and heat each spring by tens of degrees per shot, which softens it until it no longer presses.
- Measure every spring while the pack charges at a known current: one probe on the cell's − end next to the spring, the other on its plate. At 1.5 A, 15 mV is 10 mΩ and fine; 50 mV or more means the spring needs a shunt.
- Shunt a spring with about 3 cm of 22–20 AWG silicone wire from its top coil, the one on the cell, to its plate. Run it inside the coil with enough slack for the spring to compress fully; the wire then carries the current and the spring only presses. Solder the top coil quickly, as heat softens the spring.
- If a cell ever comes out, charge all four together in a standalone charger and put them back as a set. With the cells removed the BMS sees no pack and latches off; the charger unlocks it.

### Charging

The BMS is common-port: the charger and the turret share its P+/P−, so a charger plugged into the pack-side female XT60 charges the pack through the BMS. The balance lead lets the charger measure and balance each cell, which the series pack needs once the cells no longer charge one by one.

1. Unplug the XT60 pair. The turret is now off, so it cannot be armed while charging.
2. Plug the charger's XT60 lead into the pack-side female XT60 and the balance plug into the charger's 4S balance port.
3. Charge at 1–1.5 A in the mode that stops at 4.20 V per cell. On most hobby chargers that is LiPo; their LiIon mode stops at 4.10 V, which is also safe and gives a little less runtime. Never use LiHV (4.35 V).
4. Unplug the charger and plug the XT60 pair back together.

- Before the first charge, measure the balance plug with a multimeter: each pin should read 3–4.2 V above the one before it, from pin 1 (B−). The Gebildet board labels its tap pads 0V, 4.2V, 8.4V, 12.6V and 16.8V; 16.8V is B+ and the same pad as P+.
- The charger stops the charge. The BMS overcharge cut-off at about 4.25 V per cell is only the backup.
- Charge on a non-flammable surface and stay nearby.
- The balance lead has no fuse. Keep its plug capped or clipped next to the XT60 so its pins cannot short against the frame or the connectors.
- Before leaving the turret for more than a few weeks, use the charger's storage mode (about 3.8 V per cell).
- The board latches off after a short-circuit or over-current trip, and when it first sees cells. Connecting the charger for a moment unlocks it.

## Battery guard (firmware)

The firmware reads the pack every 5 s while the flywheels are idle, so spin-up sag does not count. `TurretConfig::battery` sets the thresholds:

| Pack voltage | State | Effect |
|---|---|---|
| Under 5 V | No pack: USB or the 12 V jack | Arming allowed; the ESCs have no power |
| 5–10 V | Fault: reversed or dead cell | `arm` refused, an armed turret disarms |
| 10–13.2 V (under 3.3 V per cell) | Empty | `arm` refused, an armed turret disarms |
| 13.2–14 V (under 3.5 V per cell) | Low | Logged as a warning |
| 14 V and up | Good | |

The guard only sees the whole pack. The BMS is what protects each cell.

## Signals

```
 ESP GPIO 14 (TX) ───A1┐                ┌B1── J7 4 ── TTLinker RXD ┐
 ESP GPIO 21 (RX) ───A4┤ level shifter  ├B4── J7 3 ── TTLinker TXD ├── SC15 bus (3-pin cable)
 ESP GPIO 41 ────────A2┤ (Adafruit 757) ├B2── link PUSH ── power board J4 3 ── pusher servo signal
 ESP 3V3 ────────────LV│                │HV── 5 V
 GND ───────────────GND┘                └GND─ GND

 ESP GPIO 17 ── J5 3 ── ESC left signal      (J5 1 to ESC GND)
 ESP GPIO 18 ── J6 3 ── ESC right signal     (J6 1 to ESC GND)

 Light gate (J9):  3V3 ── R3 150 Ω ── J9 1 ── IR LED ── J9 3 GND
                   3V3 ── R4 10 kΩ ──┬── GPIO 42
                                     └── J9 2 ── phototransistor C;  E ── J9 3 GND

 Arm switch (DPST, power board J3):  pole A between J3 2 (6 V) and J3 1 (pusher)
                                     pole B between J3 4 (link ARM ── GPIO 47) and J3 3 (GND)

 TFmini Plus (J8):  red 5 V, black GND, TX ── GPIO 6, RX ── GPIO 7   (3.3 V UART, no shifter)

 Battery:  divider on the power board ── link VBAT ── GPIO 1
```

- If `ping 0` gets no answer, swap J7 pins 3 and 4. The labels on these boards are not consistent about whose side they name.
- The light gate phototransistor pulls GPIO 42 down while lit. The original wired it from 5 V, which can push more than 3.3 V into the pin; the pull-up to 3V3 avoids that. The IR LED also runs from 3V3, through 150 Ω, at about 13 mA.
- The arm switch is the hardware arm interlock from the safety requirements. With pole A open the pusher has no power, so no dart can fire even if the flywheels spin. Pole B lets the firmware refuse `arm` while the switch is off and disarm when it is turned off.
- The pusher signal goes through the shifter like in the original. Most RC servos read 3.3 V as high, but some digital servos on 6 V are marginal and jitter. A BSS138 is fast enough for a 50 Hz pulse. Its 10 kΩ pull-up holds the line high until the firmware starts the PWM; servos ignore that as an invalid pulse.
- The shifter's A3/B3 channel is unused.
- The ESCs stay on 3.3 V signals, as in the original; BLHeli ESCs read them fine.
- BSS138 shifters are slow. If the servo bus gets errors at 1 Mbaud, use a 74AHCT125 buffer or lower the servo baud rate.

## Vision node

The vision node is a second Freenove board with its camera (a GC0308 on ours) on the camera connector. It sits on the tilt stage directly above the barrel (see [Sensor mounting](architecture.md#sensor-mounting-and-aim-point-open)). Only 5 V and GND run to it through the gimbal, from J10 on the power board; it has no signal wires to the controller, because it talks to the phone over its own Wi-Fi and to the controller over ESP-NOW in step 5. With Wi-Fi and the camera running, it draws up to about 500 mA, well within the 5 V buck.

| Function | GPIO | Notes |
|---|---|---|
| Camera | XCLK 15, SIOD 4, SIOC 5, D0–D7 11, 9, 8, 10, 12, 18, 17, 16, VSYNC 6, HREF 7, PCLK 13 | Freenove's (ESP32-S3-EYE) camera map, assigned in `firmware/vision/src/board.rs` |
| microSD | CMD 38, CLK 39, D0 40 | SDMMC 1-bit, FAT32. Holds the enrolled people; detection runs without a card |

## Wire gauges

Use stranded silicone wire everywhere; it survives the pan and tilt movement. Add strain relief where wires cross the gimbal axes. J3, J4 and J10 have no tie hole on the power board, so strain-relieve those wires on the tray.

| Run | AWG | Why |
|---|---|---|
| Battery → fuse → XT60 → PDB | 14 | Flywheel spin-up peaks of 30–40 A for a fraction of a second |
| PDB → each ESC | 16 | Up to ~20 A per ESC during spin-up; 18 is fine if the run is under 10 cm |
| ESC → motor phases | As supplied | Usually 18–20 AWG on the motor |
| PDB → power board J1 | 20 | Under 3 A on the 16 V side |
| J11 → TTLinker VIN (7.5 V), J3 1–2 ↔ arm switch pole A (6 V) | 20 | Up to 3.4 A while stalled |
| J10 → vision node, link GND and 5V | 22 | Under 1 A |
| Pusher servo → J4, ESCs → J5/J6 | Servo lead as supplied | |
| Link VBAT, ARM and PUSH, J3 3–4, J7, J8, J9 | 26 | Milliamps |

## First power-on

1. With the ESP out of its sockets and nothing on J4, J10 or J11, check with a meter that VIN is not shorted to GND (the MP1584s hang mirrored on the back, so a module fitted the wrong way round shorts it), and that the PDB's + and − buses are not shorted.
2. With the arm switch off, check that there is no continuity from J3 pin 2 to J4 pin 2.
3. Check the five link wires through the joints, as listed under the board README's [safety checks](../hardware/turret-pdb/README.md#safety-checks-after-assembly).
4. On the 12 V jack only, set the bucks to 7.5 V at J11, 6.0 V at J3 pin 2 and 5.0 V at J10, and lock the trimmers. J1 should read about 0 V.
5. With the cells in, measure the pack leads: 12.8–16.8 V. About 8 V means a reversed cell.
6. Plug in the ESP. First battery power-on: arm switch off, no darts, flywheel motors unplugged from the ESCs.
7. Run `B` on the console and compare with a multimeter across the pack.
8. Run `ping 0` and `ping 1`. No reply: swap J7 pins 3 and 4.
9. Run `status` and flip the arm switch; it should show on and off.
10. Calibrate both ESCs to 1000–2000 µs before the first spin-up.

## Bill of materials

| Qty | Part | Notes |
|---|---|---|
| 1 | Freenove ESP32-S3 WROOM board | Turret controller, camera connector left empty |
| 1 | Freenove ESP32-S3 WROOM CAM board | Vision node, step 4. Ships with a GC0308 sensor; an OV2640 module fits the same connector |
| 1 | microSD card, FAT32 | Vision node: enrolled people. Any size; 1 GB is plenty |
| 2 | Waveshare SC15 serial bus servo | Pan (ID 0) and tilt (ID 1) |
| 1 | Feetech TTLinker (mini) | UART to half-duplex servo bus |
| 1 | Controller boards: power and logic, JLCPCB, 2 layers, 1.6 mm | Gerbers from `hardware/turret-pdb/tools/export.sh`; see the [board README](../hardware/turret-pdb/README.md) |
| 3 | MP1584 buck module | U3 7.5 V, U4 6 V, U5 5 V. On male header pins on the power board's back. Lock the trimmers after setting them |
| 2 | SB560 Schottky diode (DO-201AD) | D1, D2: battery and bench jack OR |
| 1 | 5.5 × 2.1 mm DC jack (PJ-002A) | J2, 12 V bench input |
| 1 | 100 µF 35 V electrolytic, 6.3 mm diameter, 2.5 mm pitch | C2, on VIN |
| 1 | 1000 µF 16 V low-ESR electrolytic, 10 × 20 mm | C3, at the pusher connection J4, lying flat |
| 1 | Resistors 39 kΩ, 8.2 kΩ (1 %), 100 nF capacitor | R1, R2, C1: battery divider on the power board |
| 1 | 4-channel BSS138 level shifter (Adafruit 757) | U2 on the logic board, mounted upside down. GPIO 14 → TTLinker on A1/B1, TTLinker → GPIO 21 on A4/B4, pusher signal on A2/B2. Low side is LV and A1–A4, high side HV and B1–B4 |
| 1 | Resistors 150 Ω and 10 kΩ | R3, R4: light gate LED and pull-up on the logic board |
| – | 2 × 1×20 female header strip, 2.54 mm male headers (2 × 3-pin, 2 × 4-pin, 1 × 3-pin and pins for the MP1584s), Dupont housings | ESP sockets; J5–J9; plugs for the ESC, TTLinker, TFmini and light gate leads |
| 1 | IR LED, IR phototransistor | Light gate |
| 1 | TFmini Plus | Step 3 |
| 1 | 12 V 2.5 A adapter, 5.5 × 2.1 mm, centre positive | Optional, for bench mode |
| 2 | 2206 2300 KV brushless motor | Original uses Armattan Oomph 2206/2300 KV; any 2206–2207, 2300–2500 KV with a 5 mm shaft and 16×19 mm M3 mount should fit the printed mounts (check) |
| 1 | Aluminium flat bar 12 × 2 mm, 76 mm long | Flywheel motor plate. It slides into the 11.9 × 2.7 mm channel along the bottom of the printed flywheel holder, under both motor seats; the motors sit on it and their centre-line screws pass through it. Four 3.2 mm holes on the centre line at 7.95, 26.95, 50.45 and 66.45 mm from one end (19 mm pair for the first motor, 16 mm pair for the second). File the edges if a 12 mm bar is tight; 12 × 2.5 mm also fits, 3 mm stock does not. Where only 15 × 2 mm is sold, saw and file a piece down to width. Mount the motors with M3 × 8, so no screw reaches more than about 4 mm into the motor base and its windings |
| 2 | 30 A BLHeli_S or BLHeli_32 ESC, 2–4S | Original uses Racerstar RS30A; no BEC needed. 35 × 17 mm, 4.5 mm high at the motor end and 5.9 mm at the back, fits the printed [ESC holder](../hardware/turret-pdb/README.md#esc-holder) |
| 1 | Standard-size positional RC servo (metal gear) | Dart pusher, 6 V. MG996R class (TowerPro or a clone), 180°, stalls at about 2.5 A. Not a continuous-rotation (360°) servo, which has no position feedback |
| 4 | High-drain 18650 cell | e.g. Molicel P28A (35 A); Samsung 30Q (15 A) is the minimum |
| 1 | 4S BMS, common port, ≥30 A continuous, about 61 × 46 mm | Over-discharge, overcharge and short protection; common port, so the charger and the turret share P+/P−. Built with a Gebildet E1689 (4S 40A "Enhanced"); its own balancing is not relied on, as the charger balances |
| 1 | Printed 2×2 battery holder and covers | From the original; print the covers in PETG or ASA |
| 2 | 18650 double contact plate, spring + button | Front cover bridges, one per stack. The cells in a stack are 19.0 mm apart and the recess is 10.1 × 30.5 mm, so a spring plate and a button plate cut to fit and soldered together also work; nickel-plated steel at least 0.3 mm, or phosphor bronze. Shunt every spring (see [Battery pack](#battery-pack)) |
| 1 | 18650 spring plate + 1 button plate, joined with 14 AWG or nickel strip | Back cover bridge across the two stacks. Use one double plate instead if one matches that span |
| 1 | 18650 single spring plate | Back cover, pack − |
| 1 | 18650 single button plate | Back cover, pack + |
| 1 | Hobby balance charger, 1–6S, built-in AC supply | e.g. ToolkitRC M6AC or ISDT 608AC. Needs a mode that stops at 4.20 V per cell, a 4S JST-XH balance port and a male XT60 output lead |
| 1 | 4S JST-XH balance lead, 22 AWG silicone, about 10 cm | Soldered to the BMS tap pads. Half of a 4S balance extension works: keep the end that plugs into the charger |
| 1 | Quadcopter power distribution board, ≥3 pad pairs, 2 oz copper | e.g. Matek PDB-XT60 or a clone with 6 pad pairs. The + and − buses for the ESCs and the power board; its board XT60 and any onboard BEC stay unused |
| 1 | Inline mini blade (ATM) fuse holder, 14 AWG leads | |
| 2 | 25 A mini blade fuse | One spare. Never go above 30 A on 14 AWG |
| 1 | DPST or DPDT mini toggle with a 6 mm (M6 × 0.75) bushing, ≥3 A DC | Arm switch, in the printed bracket across the rear cradles |
| 1 | XT60 pigtail pair, 14 AWG, about 10 cm | Battery to turret, also the power switch: the male on the PDB's tab pads, the female after the fuse and BMS P−. An anti-spark XT60 if the BMS trips on plug-in |
| – | 2-part epoxy | Fixes the battery contact plates in the covers |
| – | Silicone wire 14, 16, 20, 22, 26 AWG; heat shrink | |
| – | Stack tray: printed tray, deck, 4 risers and arm switch bracket in PLA; M3 screws 4 × 16 and 4 × 8 with 8 nuts, 4 × 20, 4 × 6; 4 M3 17 mm female–female standoffs; M3 washers; six 4.8 mm zip ties and a few small ones; 1 mm double-sided foam tape | Holds both stacks under the tubes; see the [stack tray](../hardware/turret-pdb/README.md#stack-tray) |
| – | ESC holders: left and right printed, 4 M3 × 6 socket head screws, 4 small zip ties, PLA or PETG filament | Replace the original's ESC holders; the ESCs snap in. See the [ESC holder](../hardware/turret-pdb/README.md#esc-holder) |
| – | Battery shims, optional: printed in PETG or ASA | Only if the front cover will not close; see [Battery pack](#battery-pack) |
| – | 16 mm carbon tube ×1, 12 mm ×2, M3 screws | Frame, from the original |
