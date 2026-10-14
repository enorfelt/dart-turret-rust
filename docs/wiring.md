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
| Pusher servo | 41 | 39 | Through the level shifter (channel 3). JTAG MTDI, free while debugging over USB |
| Light gate | 42 | 38 | JTAG MTMS, free while debugging over USB |
| Battery divider | 1 | 16 | ADC1_CH0, so readings keep working with Wi-Fi / ESP-NOW on |
| TFmini Plus RX (ESP receives) | 6 | – | UART2, step 3 |
| TFmini Plus TX (ESP sends) | 7 | – | UART2, step 3 |
| Arm switch sense | 47 | – | Internal pull-up; switch closes to GND when armed |

Avoid: 0, 3, 45, 46 (strapping), 2 (LED), 48 (WS2812), 19/20 (USB), 43/44 (UART0 to the onboard USB-UART), 35–40.

## Power

```
 4S1P 18650 pack (+ 4S BMS)
   +  ──[ 25 A blade fuse ]──[ main switch ]──┬────────────────────────────────── + bus (14 AWG)
   −  ────────────────────────────────────────┼────────────────────────────────── − bus (14 AWG)
                                              │
     + bus / − bus (keep short) feeds:        │
       ├── ESC left   (16 AWG) ─── motor left   (3 phase)
       ├── ESC right  (16 AWG) ─── motor right  (3 phase)
       ├── Buck 7.5 V, ≥5 A (20 AWG) ─── TTLinker VIN ─── SC15 pan ─── SC15 tilt
       ├── Buck 6 V,   ≥3 A (20 AWG) ─── arm switch pole A ─── pusher servo +
       ├── Buck 5 V,   ≥2 A (22 AWG) ─── ESP 5V pin, TFmini 5 V, level shifter HV,
       │                                  TTLinker 5 V, light gate LED
       └── 39 kΩ ──┬── 8.2 kΩ ── − bus        (26 AWG)
                   ├── 100 nF ── − bus
                   └── GPIO 1
```

- The fuse goes as close to the battery + as possible, so it protects everything downstream, the switch included.
- The divider sits after the switch so its 0.36 mA does not drain the pack while off.
- All grounds are common: buck outputs, ESC signal grounds, ESP GND, TTLinker GND, TFmini GND.
- Connect only signal and ground from the ESC servo leads. If an ESC has a BEC, leave its red wire unconnected.
- The SC15 is rated 4.8–8.4 V and stalls at 1.7 A, so two stalled servos need 3.4 A. The original 8 V MP1584 (3 A peak) is undersized for that.
- Turn the turret off before plugging in USB, unless the board's schematic shows that VBUS and the 5V pin are isolated by a diode.

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
| Pack + | Back end plate, top-right | BMS B+, then the fuse | 14 |
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
 ESP GPIO 14 (TX) ──LV1┐                ┌HV1── TTLinker RXD ┐
 ESP GPIO 21 (RX) ──LV2┤ level shifter  ├HV2── TTLinker TXD ├── SC15 bus (3-pin cable)
 ESP 3V3 ───────────LV │  (BSS138, 4ch) │HV ── 5 V          │
 GND ───────────────GND┘                └GND── GND          ┘

 ESP GPIO 17 ─────────── ESC left signal      (ESC GND to common GND)
 ESP GPIO 18 ─────────── ESC right signal
 ESP GPIO 41 ──LV3┤ level shifter ├HV3── pusher servo signal   (same shifter, channel 3)

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

## Wire gauges

Use stranded silicone wire everywhere; it survives the pan and tilt movement. Add strain relief where wires cross the gimbal axes.

| Run | AWG | Why |
|---|---|---|
| Battery → fuse → switch → + / − bus | 14 | Flywheel spin-up peaks of 30–40 A for a fraction of a second |
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
| 2 | Waveshare SC15 serial bus servo | Pan (ID 0) and tilt (ID 1) |
| 1 | Feetech TTLinker (mini) | UART to half-duplex servo bus |
| 1 | 4-channel BSS138 level shifter | 3.3 V ↔ 5 V for the TTLinker (channels 1–2) and the pusher signal (channel 3) |
| 2 | 2206 2300 KV brushless motor | Original uses Armattan Oomph 2206/2300 KV; any 2206–2207, 2300–2500 KV with a 5 mm shaft and 16×19 mm M3 mount should fit the printed mounts (check) |
| 2 | 30 A BLHeli_S or BLHeli_32 ESC, 2–4S | Original uses Racerstar RS30A; no BEC needed |
| 1 | Standard-size RC servo (metal gear) | Dart pusher, 6 V |
| 4 | High-drain 18650 cell | e.g. Molicel P28A (35 A); Samsung 30Q (15 A) is the minimum |
| 1 | 4S BMS, ≥30 A continuous | Over-discharge and short protection. No balancing needed, as the cells are charged out of the holder; prefer one that recovers by itself when cells are reinserted |
| 1 | Printed 2×2 battery holder and covers | From the original; print the covers in PETG or ASA |
| 2 | 18650 double contact plate, spring + button | Front cover bridges, one per stack. Match the plate's cell pitch to the holder's stack spacing; nickel-plated steel at least 0.3 mm, or phosphor bronze |
| 1 | 18650 spring plate + 1 button plate, joined with 14 AWG or nickel strip | Back cover bridge across the two stacks. Use one double plate instead if one matches that span |
| 1 | 18650 single spring plate | Back cover, pack − |
| 1 | 18650 single button plate | Back cover, pack + |
| 1 | 4-bay Li-ion charger, 4.2 V per cell | e.g. Nitecore or XTAR; charges each cell separately |
| 1 | Buck converter 7.5 V, ≥5 A | e.g. Pololu D36V50F7, or an adjustable XL4015 5 A set to 7.5 V |
| 1 | Buck converter 6 V, ≥3 A | e.g. Pololu D36V28F6, or an MP1584 for a small pusher servo |
| 1 | Buck converter 5 V, ≥2 A | e.g. Pololu D36V28F5, or an MP1584 set to 5 V |
| 1 | Inline mini blade (ATM) fuse holder, 14 AWG leads | |
| 2 | 25 A mini blade fuse | One spare. Never go above 30 A on 14 AWG |
| 1 | Main power switch | An RC anti-spark electronic switch, 2–6S, ≥40 A, avoids the spark from the ESC capacitors. A mechanical switch must be rated ≥20 A at ≥24 V DC |
| 1 | DPST toggle or key switch, ≥3 A DC | Arm switch |
| 1 | XT60 connector pair | Battery to turret |
| 1 | Resistors 39 kΩ, 8.2 kΩ (1 %), 100 nF capacitor | Battery divider |
| 1 | IR LED, IR phototransistor, 330 Ω, 10 kΩ | Light gate |
| 1 | TFmini Plus | Step 3 |
| – | 2-part epoxy | Fixes the battery contact plates in the covers |
| – | Silicone wire 14, 16, 20, 22, 26 AWG; heat shrink; JST/Dupont connectors | |
| – | 16 mm carbon tube ×1, 12 mm ×2, M3 screws | Frame, from the original |
