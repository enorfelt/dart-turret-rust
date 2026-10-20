# Wiring

Turret controller wiring for a Freenove ESP32-S3 WROOM board (ESP32-S3-WROOM-1, octal PSRAM) on a 4S1P 18650 pack. Based on the [original Dart-Turret wiring](https://github.com/Tops76/Dart-Turret), with a main fuse, a power switch, a hardware arm switch, beefier servo supplies and pins moved for the Freenove board.

Status: proposed, not wired yet. `firmware/turret/src/board.rs` uses this pin map.

The same wiring drawn as colour-coded schematics is in [wiring.html](wiring.html); open it in a browser. Keep the two in step when the wiring changes.

## Pin map (Freenove ESP32-S3 WROOM)

The original pins do not fit this board: GPIO 33–37 are taken by the octal PSRAM (33/34 are not even broken out), and GPIO 38–40 go to the SD card slot. The camera connector stays empty on the turret controller, so camera pins are free to use.

| Function | GPIO | Was | Notes |
|---|---|---|---|
| Servo bus TX (UART1) | 14 | 33 | Through the level shifter to the TTLinker |
| Servo bus RX (UART1) | 21 | 34 | Through the level shifter to the TTLinker |
| ESC left | 17 | 17 | Camera pin, unused here |
| ESC right | 18 | 18 | Camera pin, unused here |
| Pusher servo | 41 | 39 | Through the level shifter (A3/B3). JTAG MTDI, free while debugging over USB |
| Light gate | 42 | 38 | JTAG MTMS, free while debugging over USB |
| Battery divider | 1 | 16 | ADC1_CH0, so readings keep working with Wi-Fi / ESP-NOW on |
| TFmini Plus RX (ESP receives) | 6 | – | UART2, step 3 |
| TFmini Plus TX (ESP sends) | 7 | – | UART2, step 3 |
| Arm switch sense | 47 | – | Internal pull-up; switch closes to GND when armed |

Avoid: 0, 3, 45, 46 (strapping), 2 (LED), 48 (WS2812), 19/20 (USB), 43/44 (UART0 to the onboard USB-UART), 35–40.

## Power

```
 4S1P 18650 pack (+ 4S BMS)
   +  ──[ 25 A blade fuse ]──[ XT60 + ]───────┬────────────────────────────────── + bus (PDB)
   −  ──────────────────────[ XT60 − ]────────┼────────────────────────────────── − bus (PDB)
                                              │
     + bus / − bus (keep short) feeds:        │
       ├── ESC left   (16 AWG) ─── motor left   (3 phase)
       ├── ESC right  (16 AWG) ─── motor right  (3 phase)
       ├── Buck 7.5 V, ≥5 A (20 AWG) ─── TTLinker VIN ─── SC15 pan ─── SC15 tilt
       ├── Buck 6 V,   ≥3 A (20 AWG) ─── arm switch pole A ──┬── pusher servo +
       │                                                     └── + 1000 µF − ── pusher servo −
       ├── Buck 5 V,   ≥2 A (22 AWG) ─── ESP 5V pin, TFmini 5 V, level shifter HV,
       │                                  TTLinker 5 V, light gate LED,
       │                                  vision board 5V pin (through the gimbal)
       └── 39 kΩ at PDB VCC pad ──(26 AWG)──┬── 8.2 kΩ ── − bus
                                            ├── 100 nF ── − bus
                                            └── GPIO 1
```

- The XT60 is the power switch: unplug it to turn the turret off. At 4S the spark from the ESC input capacitors is small; plug in quickly and firmly, without wiggling.
- The ESC capacitor inrush on plug-in can trip the BMS short-circuit protection. Plug in a few times on the bench; if the turret ever stays dark, use a BMS that recovers by itself or an anti-spark XT60 with a pre-charge resistor.
- The fuse goes as close to the battery + as possible, on the pack side of the XT60, so it protects everything downstream.
- The + and − buses are a quadcopter power distribution board (PDB). Every + pad on it is the same copper, and so is every − pad, so each lead in the PDB pads table below can go on any pad of the right polarity. Soldered copper adds well under 1 mΩ and does not loosen from flywheel vibration.
- Without a PDB, use two Wago 221-415 lever connectors per rail, joined by a short 14 AWG jumper. Each rail has six leads (feed, two ESCs, three bucks) and a 221-415 takes five. Solder the 26 AWG divider wires to a buck input lead instead, as they are below the Wago's minimum. Do not use screw terminals on bare stranded wire, as they loosen under vibration.
- Solder the 39 kΩ directly to the PDB VCC pad and put the 8.2 kΩ and 100 nF next to the ESP. The 26 AWG wire then carries only the divided voltage, so a chafed wire shorted to ground draws under 0.5 mA instead of burning before the 25 A fuse blows. The 100 nF filters noise picked up on the long wire.
- The divider sits on the turret side of the XT60 so its 0.36 mA does not drain the pack while unplugged.
- All grounds are common: buck outputs, ESC signal grounds, ESP GND, TTLinker GND, TFmini GND, vision board GND.
- Connect only signal and ground from the ESC servo leads. If an ESC has a BEC, leave its red wire unconnected.
- The SC15 is rated 4.8–8.4 V and stalls at 1.7 A, so two stalled servos need 3.4 A. The original 8 V MP1584 (3 A peak) is undersized for that.
- The pusher servo (MG996R class) stalls at about 2.5 A and draws spikes of that size every time it starts to move. A 1000 µF, ≥16 V low-ESR electrolytic capacitor across its leads supplies those spikes, so they do not sag the 6 V rail or bounce the shared ground. Solder it at the servo end, after pole A: + to the servo's red lead, − to its brown lead. Check the polarity, as a reversed electrolytic heats up and can burst.
- With pole A open, the capacitor holds enough charge to run the servo for a few milliseconds, far too short to push a dart, so the arm switch still cuts the pusher.
- Unplug the XT60 before plugging in USB, unless the board's schematic shows that VBUS and the 5V pin are isolated by a diode.

### PDB pads

| PDB pad | Connect |
|---|---|
| Battery input (the two large holes) | XT60 pigtail, 14 AWG |
| Pad pair 1 | ESC left, 16 AWG |
| Pad pair 2 | ESC right, 16 AWG |
| Pad pair 3 | 7.5 V buck input, 20 AWG |
| Pad pair 4 | 6 V buck input, 20 AWG |
| Pad pair 5 | 5 V buck input, 20 AWG |
| Pad pair 6 | Spare |
| Small VCC pad | 39 kΩ of the battery divider, soldered directly to the pad |
| Small GND pad | Empty: the divider's 8.2 kΩ and 100 nF go to ESP GND |
| 5 V / 12 V BEC pads | Empty: the PDB's own regulators are not used |

## Battery pack

The printed holder takes the four cells as two stacks of two, lying front to back. The covers hold contact plates, so the plates make the series links and no wire runs between cells. The cells come out for charging in a standalone 18650 charger, which charges each cell to 4.2 V on its own, so the pack needs no balance lead.

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

- Pack leads and B2 leave through the hole in the back cover. The front cover has no hole: drill a 3 mm hole or file a notch for B1 and B3, and run them back along the frame.
- The back bridge link carries the full pack current, like the pack leads. Lay it in the long channel of the back cover.
- Solder to the plate tabs before the plates go into the covers, with the cells out. Nickel-plated steel takes solder with a hot iron and flux.
- Let the cover slots locate the plates; test-fit a plate before gluing. Then fix them with 2-part epoxy on the edges and tabs only, never on the contact face or the spring coils. Avoid hot glue, which softens near 60 °C, and superglue, which cracks with repeated cell swaps. Heat-staking the plate over a printed pin also works.
- Strain-relieve every wire on the cover a few cm from its solder joint, with a zip tie through a printed loop or epoxy over the wire. Pulled wires are what tear plates loose.
- After assembly, every spring should be visibly compressed, the covers should close without bulging, and the pack should read 12.8–16.8 V.
- Mark + and − next to every pocket. A reversed cell makes the pack read about 8 V; the firmware then refuses `arm` and reports a battery fault.
- Use unprotected flat-top or button-top high-drain cells. Protected cells are longer and limit the current far below the spin-up peak.
- The 8 contacts in the chain are the weak point: at a 35 A spin-up peak, about 10 mΩ each sags the pack by almost 3 V. Use thick plates, print the covers in PETG or ASA rather than PLA, and measure the drop across a contact during a spin-up; more than about 0.1 V means that contact needs attention.
- Charge all four cells together and put them back as a set. Store them at about 3.7 V per cell.
- With the cells removed, the BMS sees no pack. Some BMS boards stay latched off afterwards until a charger is connected to P+/P−; prefer one that recovers by itself.

## Battery guard (firmware)

The firmware reads the pack every 5 s while the flywheels are idle, so spin-up sag does not count. `TurretConfig::battery` sets the thresholds:

| Pack voltage | State | Effect |
|---|---|---|
| Under 5 V | No pack, USB power | Arming allowed; nothing downstream has power |
| 5–10 V | Fault: reversed or dead cell | `arm` refused, an armed turret disarms |
| 10–13.2 V (under 3.3 V per cell) | Empty | `arm` refused, an armed turret disarms |
| 13.2–14 V (under 3.5 V per cell) | Low | Logged as a warning |
| 14 V and up | Good | |

The guard only sees the whole pack. The BMS is what protects each cell.

## Signals

```
 ESP GPIO 14 (TX) ───A1┐                ┌B1─── TTLinker RXD ┐
 ESP GPIO 21 (RX) ───A2┤ level shifter  ├B2─── TTLinker TXD ├── SC15 bus (3-pin cable)
 ESP 3V3 ───────────LV │  (BSS138, 4ch) │HV ── 5 V          │
 GND ───────────────GND┘                └GND── GND          ┘

 ESP GPIO 17 ─────────── ESC left signal      (ESC GND to common GND)
 ESP GPIO 18 ─────────── ESC right signal
 ESP GPIO 41 ───A3┤ level shifter ├B3─── pusher servo signal   (same shifter, A3/B3)

 Light gate:  5 V ── 330 Ω ── IR LED ── GND
              3V3 ── 10 kΩ ──┬── GPIO 42
                             └── phototransistor C;  E ── GND

 Arm switch (DPST):  pole A in series with the pusher servo's 6 V
                     pole B: GPIO 47 ── switch ── GND

 TFmini Plus:  red 5 V, black GND, TX ── GPIO 6, RX ── GPIO 7   (3.3 V UART, no shifter)
```

- If `ping 0` gets no answer, swap TTLinker RXD/TXD. The labels on these boards are not consistent about whose side they name.
- The light gate phototransistor pulls GPIO 42 down while lit. The original wired it from 5 V, which can push more than 3.3 V into the pin; the pull-up to 3V3 avoids that.
- The arm switch is the hardware arm interlock from the safety requirements. With pole A open the pusher has no power, so no dart can fire even if the flywheels spin. Pole B lets the firmware refuse `arm` while the switch is off and disarm when it is turned off.
- The pusher signal goes through the shifter like in the original. Most RC servos read 3.3 V as high, but some digital servos on 6 V are marginal and jitter. A BSS138 is fast enough for a 50 Hz pulse. Its 10 kΩ pull-up holds the line high until the firmware starts the PWM; servos ignore that as an invalid pulse.
- The ESCs stay on 3.3 V signals, as in the original; BLHeli ESCs read them fine.
- BSS138 shifters are slow. If the servo bus gets errors at 1 Mbaud, use a 74AHCT125 buffer or lower the servo baud rate.

## Vision node

The vision node is a second Freenove board with its camera (a GC0308 on ours) on the camera connector. It sits on the tilt stage directly above the barrel (see [Sensor mounting](architecture.md#sensor-mounting-and-aim-point-open)). Only 5 V and GND run to it through the gimbal; it has no signal wires to the controller, because it talks to the phone over its own Wi-Fi and to the controller over ESP-NOW in step 5. With Wi-Fi and the camera running, it draws up to about 500 mA, well within the 5 V buck.

| Function | GPIO | Notes |
|---|---|---|
| Camera | XCLK 15, SIOD 4, SIOC 5, D0–D7 11, 9, 8, 10, 12, 18, 17, 16, VSYNC 6, HREF 7, PCLK 13 | Freenove's (ESP32-S3-EYE) camera map, assigned in `firmware/vision/src/board.rs` |
| microSD | CMD 38, CLK 39, D0 40 | SDMMC 1-bit, FAT32. Holds the enrolled people; detection runs without a card |

## Wire gauges

Use stranded silicone wire everywhere; it survives the pan and tilt movement. Add strain relief where wires cross the gimbal axes.

| Run | AWG | Why |
|---|---|---|
| Battery → fuse → XT60 → + / − bus | 14 | Flywheel spin-up peaks of 30–40 A for a fraction of a second |
| Bus → each ESC | 16 | Up to ~20 A per ESC during spin-up; 18 is fine if the run is under 10 cm |
| ESC → motor phases | As supplied | Usually 18–20 AWG on the motor |
| Bus → buck converter inputs | 20 | Under 3 A on the 16 V side |
| 7.5 V servo bus, 6 V pusher | 20 | Up to 3.4 A while stalled |
| 5 V logic rail | 22 | Under 1 A |
| Signals, divider, light gate, arm sense | 26 | Milliamps |

## Bill of materials

| Qty | Part | Notes |
|---|---|---|
| 1 | Freenove ESP32-S3 WROOM board | Turret controller, camera connector left empty |
| 1 | Freenove ESP32-S3 WROOM CAM board | Vision node, step 4. Ships with a GC0308 sensor; an OV2640 module fits the same connector |
| 1 | microSD card, FAT32 | Vision node: enrolled people. Any size; 1 GB is plenty |
| 2 | Waveshare SC15 serial bus servo | Pan (ID 0) and tilt (ID 1) |
| 1 | Feetech TTLinker (mini) | UART to half-duplex servo bus |
| 1 | 4-channel BSS138 level shifter (Adafruit 757) | 3.3 V ↔ 5 V for the TTLinker (A1/B1, A2/B2) and the pusher signal (A3/B3). Low side is LV and A1–A4, high side HV and B1–B4 |
| 2 | 2206 2300 KV brushless motor | Original uses Armattan Oomph 2206/2300 KV; any 2206–2207, 2300–2500 KV with a 5 mm shaft and 16×19 mm M3 mount should fit the printed mounts (check) |
| 2 | 30 A BLHeli_S or BLHeli_32 ESC, 2–4S | Original uses Racerstar RS30A; no BEC needed |
| 1 | Standard-size positional RC servo (metal gear) | Dart pusher, 6 V. MG996R class (TowerPro or a clone), 180°, stalls at about 2.5 A. Not a continuous-rotation (360°) servo, which has no position feedback |
| 1 | 1000 µF ≥16 V low-ESR electrolytic capacitor | Across the pusher servo's supply, at the servo |
| 4 | High-drain 18650 cell | e.g. Molicel P28A (35 A); Samsung 30Q (15 A) is the minimum |
| 1 | 4S BMS, ≥30 A continuous | Over-discharge and short protection. No balancing needed, as the cells are charged out of the holder; prefer one that recovers by itself when cells are reinserted |
| 1 | Printed 2×2 battery holder and covers | From the original; print the covers in PETG or ASA |
| 2 | 18650 double contact plate, spring + button | Front cover bridges, one per stack. Match the plate's cell pitch to the holder's stack spacing; nickel-plated steel at least 0.3 mm, or phosphor bronze |
| 1 | 18650 spring plate + 1 button plate, joined with 14 AWG or nickel strip | Back cover bridge across the two stacks. Use one double plate instead if one matches that span |
| 1 | 18650 single spring plate | Back cover, pack − |
| 1 | 18650 single button plate | Back cover, pack + |
| 1 | 4-bay Li-ion charger, 4.2 V per cell | e.g. Nitecore or XTAR; charges each cell separately |
| 1 | Buck converter 7.5 V, ≥5 A | e.g. Pololu D36V50F7, or an adjustable XL4015 5 A set to 7.5 V |
| 1 | Buck converter 6 V, ≥3 A | e.g. Pololu D36V28F6. An MP1584 module (3 A peak) handles normal pushes with the 1000 µF capacitor, but overheats and cuts out during a sustained stall; lock its trim pot after setting it, as above 7.2 V the MG996R is damaged |
| 1 | Buck converter 5 V, ≥2 A | e.g. Pololu D36V28F5, or an MP1584 set to 5 V |
| 1 | Quadcopter power distribution board, ≥5 pad pairs plus a VCC / GND sense pad, 2 oz copper | e.g. Matek PDB-XT60 or a clone with 6 pad pairs. The + and − buses; any onboard BEC stays unused. Two Wago 221-415 per rail work as a solderless fallback |
| 1 | Inline mini blade (ATM) fuse holder, 14 AWG leads | |
| 2 | 25 A mini blade fuse | One spare. Never go above 30 A on 14 AWG |
| 1 | DPST toggle or key switch, ≥3 A DC | Arm switch |
| 1 | XT60 connector pair | Battery to turret, also the power switch. An anti-spark XT60 if the BMS trips on plug-in |
| 1 | Resistors 39 kΩ, 8.2 kΩ (1 %), 100 nF capacitor | Battery divider |
| 1 | IR LED, IR phototransistor, 330 Ω, 10 kΩ | Light gate |
| 1 | TFmini Plus | Step 3 |
| – | 2-part epoxy | Fixes the battery contact plates in the covers |
| – | Silicone wire 14, 16, 20, 22, 26 AWG; heat shrink; JST/Dupont connectors | |
| – | 16 mm carbon tube ×1, 12 mm ×2, M3 screws | Frame, from the original |
