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

| Function | Interface | Notes |
|---|---|---|
| Pan/tilt | 2x Waveshare SC15 on UART1, 1 Mbaud, Feetech SCS protocol, IDs 0 (pan) / 1 (tilt) | `REG_WRITE` both, then broadcast `ACTION` so the axes move together |
| Flywheels | 2x ESC, servo-style pulses via LEDC (1 MHz resolution) | Original used 500–1000 µs at ~333 Hz; recalibrate the ESCs |
| Pusher | RC servo via LEDC | Original: retracted ≈ 125°, pushed ≈ 5–10° on a 500–1000 µs scale |
| Range | TFmini Plus on UART2, 115200 8N1 | 9-byte frames `0x59 0x59 distL distH strL strH tempL tempH checksum`, 0.1–12 m |
| Dart speed | IR light gate on a GPIO interrupt | Gate blocked time over a 70 mm dart gives the muzzle velocity |
| Battery | 4S Li-ion through a 39k/8k2 divider to an ADC pin | 16.8 V reads ≈ 2.92 V |
| Manual control | BLE peripheral compatible with the existing App Inventor app | See [BLE protocol](#ble-protocol) |

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
2. Turret actuators (SC15 gimbal, ESCs, pusher, light gate, battery) and BLE compatibility with the phone app.
3. TFmini Plus driver and range-based elevation table.
4. Vision node: camera, esp-dl face detection and recognition, enrollment.
5. ESP-NOW link, autonomous mode and the safety gates above.
