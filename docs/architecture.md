# Architecture

Status: design proposal. Decisions marked *open* are settled in the roadmap step that needs them.

The mechanical design is the [Dart-Turret](https://www.printables.com/model/338574) (2-axis gimbal, two brushless flywheels, servo dart pusher), printed unchanged. The firmware is rewritten in Rust on ESP-IDF (`esp-idf-svc`), and autonomous targeting is added on top of the original manual BLE control.

## Nodes

```
        ┌──────────────────────────┐   ESP-NOW (postcard, turret-proto)   ┌──────────────────────────┐
        │ Vision node (ESP32-S3)   │ ───────────── targets ─────────────▶ │ Turret controller (S3)   │
        │ camera + PSRAM           │ ◀──────────── status ─────────────── │                          │
        │ face detect / recognize  │                                      │ gimbal, flywheels,       │
        └──────────────────────────┘                                      │ pusher, lidar, BLE       │
                                                                          └────────────┬─────────────┘
                                                                                       │ BLE (NUS-style)
                                                                                ┌──────┴──────┐
                                                                                │ Phone app   │
                                                                                └─────────────┘
```

### Turret controller (`firmware/turret`)

| Function | GPIO | Interface | Notes |
|---|---|---|---|
| Pan/tilt | TX 33, RX 34 | 2x Waveshare SC15 on UART1, 1 Mbaud, Feetech SCS protocol, IDs 0 (pan) / 1 (tilt) | `REG_WRITE` both, then broadcast `ACTION` so the axes move together; positions clamped to 0–1023 until the mechanical limits are measured |
| Flywheels | 17, 18 | 2x ESC, 1000–2000 µs at 50 Hz via LEDC (14-bit) | Standard RC range, unlike the original 500–1000 µs; **calibrate the ESCs to it before the first spin-up** |
| Pusher | 39 | RC servo, 50 Hz via LEDC | Retracted 847 µs, extended 514 µs: the original's positions converted to µs, to be checked on the printed pusher |
| Range | – | TFmini Plus on UART2, 115200 8N1 | 9-byte frames `0x59 0x59 distL distH strL strH tempL tempH checksum`, 0.1–12 m (step 3) |
| Dart speed | 38 | IR light gate, any-edge GPIO interrupt | Time between the two edges over a 70 mm dart gives km/h |
| Battery | 16 | 4S Li-ion through a 39k/8k2 divider to ADC2_CH5 | 16.8 V reads ≈ 2.92 V. ADC2 cannot be read while Wi-Fi is running, so this must move to an ADC1 pin (GPIO 1–10) before ESP-NOW |
| Manual control | – | BLE peripheral compatible with the existing App Inventor app (step 2b) | See [BLE protocol](#ble-protocol) |

Pins are assigned in one place, `firmware/turret/src/board.rs`; GPIO 38 and 16 are provisional until the turret is wired. Timings, pulse ranges, gimbal limits and battery constants live in `TurretConfig` (`crates/turret-core/src/config.rs`).

### Firmware structure

Hardware-independent logic is in `crates/turret-core` and unit-tested on the host: the SCS protocol, pulse/duty maths, battery maths, the command parser and the fire sequencer. The firmware runs one 5 ms control loop that owns all actuators. A console thread reads lines from USB-Serial-JTAG and sends parsed commands over a channel, and the light gate ISR only stores timestamps in atomics.

The fire sequencer is a non-blocking state machine: spin up → push → retract → (next shot) → linear spin-down → cooldown. It refuses to fire while disarmed, busy, cooling down, or with zero speed. Bursts and auto fire are capped at `max_shots`. Disarm cuts the throttle and retracts the pusher immediately.

### Development console

Open with `espflash monitor` (or `cargo run`). Typed characters are echoed; Enter sends the line.

| Command | Effect |
|---|---|
| `help` | List commands |
| `arm` / `disarm` | Allow / stop flywheels and pusher (boots disarmed) |
| `status` | Arming, fire phase, battery, last gimbal target |
| `ping <id>` | Ping a serial bus servo |
| `P;pan;tilt;vpan;vtilt` | Move the gimbal (same format as the app) |
| `F;1;mode;speed;burst` | Fire; mode 0 single, 1 burst, 2 auto; speed 0–180 as in the app |
| `F;0` | Cease fire after the current shot |
| `B` | Battery voltage and percent |

### Vision node (`firmware/vision`, not created yet)

An ESP32-S3 camera board with PSRAM (XIAO ESP32S3 Sense, ESP32-S3-EYE or similar). Face detection and recognition use Espressif's esp-dl / ESP-WHO models, linked into the Rust app as an ESP-IDF component through FFI. Output per frame: target box in normalized image coordinates, authorized/unauthorized, and confidence. Enrolled face embeddings stay on the device.

### Inter-node link (*open*)

Default: ESP-NOW with `turret-proto` messages encoded by postcard. That needs no cable through the gimbal and has low latency without a router. If the camera ends up wired next to the controller, a UART link carries the same messages instead.

## Targeting loop

1. The vision node reports a face box and its classification.
2. The aim point is placed **below** the face box (torso). The turret never aims at a face.
3. The pixel error from the image center drives the pan/tilt controller.
4. The TFmini Plus range gives elevation compensation from an empirically measured table.
5. All safety gates pass → spin up flywheels → push one dart → measure speed → ramp down.

## Safety requirements

- A physical arm switch must be on to spin flywheels; BLE disarm and kill work at any time.
- No fire on uncertainty: unknown, low-confidence or authorized faces are never engaged.
- Torso aim offset; no fire when the aim point cannot be placed below the face.
- Minimum engagement range from the lidar; no fire without a valid range reading.
- Rate limit between shots, and a timeout that stops the flywheels.
- Firing logic never blocks the control loop (the original did).

## BLE protocol

Kept compatible with `app_inventor/DartDroid_bluetooth_UART.aia` from the original repo.

- Device name `DART Turret`, service UUID `0xDEAD`.
- RX (write) `6E400002-B5A3-F393-E0A9-E50E24DCCA9E`, TX (notify) `6E400003-…`, TXPOS (notify) `6E400005-…`.
- ASCII, `;`-separated:

| Command | Format | Replies |
|---|---|---|
| Move | `P;<x>;<y>;<speed_x>;<speed_y>` | `P;Busy`, then `P;Ready` on TXPOS |
| Fire | `F;<fire>;<mode>;<speed>;<burst>;<auto_speed>` (mode 0 semi, 1 burst, 2 auto) | `F;Busy`, then `F;Ready;<km/h>` or `F;Ready;ERR` |
| Battery | `B…` | `B;<percent>` (the app expects a `B-` prefix; fix one side) |
| Unknown | anything else | `X;Error` |

## Original firmware pin map (Arduino, ESP32-S3)

| GPIO | Use |
|---|---|
| 17, 18 | ESC 1, ESC 2 |
| 39 | Pusher servo |
| 33 / 34 | UART1 TX / RX to the SC15 bus |
| 16 | Light gate in code, battery divider in the wiring diagram (light gate is IO38 there) |
| 1, 2, 3, 5, 6, 10 | GC9107 LCD (RST, MOSI, SCK, CS, DC, backlight) on the LilyGO T-QT |

## Known defects in the original, not to be ported

- Hardware timer 0 is shared by the fire timeout and the servo pulse generator.
- The timer ISR prints to `Serial`.
- The ESC ramp-down subtracts 5 from an unsigned value and can wrap.
- Firing blocks the BLE loop for 1–2 s.
- Command strings are shared between the BLE callback and the loop without locking.
- The battery level is hard-coded to 50 %.

## Roadmap

1. Repository, toolchain and Claude Code setup, `turret-proto` heartbeat. *(done)*
2. a. Turret drivers (SC15 gimbal, ESCs, pusher, light gate, battery) and the fire sequencer, driven from the USB console. *(done, bench-tested without actuators)*
   b. BLE peripheral compatible with the phone app, reusing the console's command parser.
3. TFmini Plus driver and range-based elevation table.
4. Vision node: camera, esp-dl face detection and recognition, enrollment.
5. ESP-NOW link, autonomous mode and the safety gates above.
