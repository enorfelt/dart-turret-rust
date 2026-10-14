# Architecture

Status: design proposal. Decisions marked *open* are settled in the roadmap step that needs them.

The mechanical design is the [Dart-Turret](https://www.printables.com/model/338574) (2-axis gimbal, two brushless flywheels, servo dart pusher), printed unchanged. The firmware is rewritten in Rust on ESP-IDF (`esp-idf-svc`), manual control moves to our own BLE protocol and phone app, and autonomous targeting is added on top.

## Nodes

```
        ┌──────────────────────────┐   ESP-NOW (postcard, turret-proto)   ┌──────────────────────────┐
        │ Vision node (ESP32-S3)   │ ───────────── targets ─────────────▶ │ Turret controller (S3)   │
        │ camera + PSRAM           │ ◀──────────── status ─────────────── │                          │
        │ face detect / recognize  │                                      │ gimbal, flywheels,       │
        └────────────┬─────────────┘                                      │ pusher, lidar, BLE       │
                     │ Wi-Fi softAP                                       └────────────┬─────────────┘
                     │ video, display only                                             │ BLE (bonded, binary)
                     │                                                          ┌──────┴──────┐
                     └─────────────────────────────────────────────────────────▶│ Phone PWA   │
                                                                                └─────────────┘
```

### Turret controller (`firmware/turret`)

| Function | GPIO | Interface | Notes |
|---|---|---|---|
| Pan/tilt | TX 14, RX 21 | 2x Waveshare SC15 on UART1 through a level shifter and a Feetech TTLinker, 1 Mbaud, Feetech SCS protocol, IDs 0 (pan) / 1 (tilt) | `REG_WRITE` both, then broadcast `ACTION` so the axes move together; positions clamped to 0–1023 until the mechanical limits are measured |
| Flywheels | 17, 18 | 2x ESC, 1000–2000 µs at 50 Hz via LEDC (14-bit) | Standard RC range, unlike the original 500–1000 µs; **calibrate the ESCs to it before the first spin-up** |
| Pusher | 41 | RC servo, 50 Hz via LEDC | Retracted 847 µs, extended 514 µs: the original's positions converted to µs, to be checked on the printed pusher |
| Range | RX 6, TX 7 | TFmini Plus on UART2, 115200 8N1 | 9-byte frames `0x59 0x59 distL distH strL strH tempL tempH checksum`, 0.1–12 m (step 3) |
| Dart speed | 42 | IR light gate, any-edge GPIO interrupt | Time between the two edges over a 70 mm dart gives km/h |
| Battery | 1 | 4S Li-ion through a 39k/8k2 divider to ADC1_CH0 | 16.8 V reads ≈ 2.92 V. ADC1, so readings keep working with Wi-Fi and ESP-NOW on |
| Arm switch | 47 | Input with pull-up, closed to GND when on | Second pole of the switch cuts the pusher servo's supply. `arm` is refused while it is off, and turning it off disarms; on needs 50 ms without bounce |
| Manual control | – | BLE peripheral (NimBLE), bonded with a passkey | See [BLE protocol](#ble-protocol) |

The controller board is a Freenove ESP32-S3 WROOM (octal PSRAM, so GPIO 33–37 are unavailable and 38–40 go to the SD slot). Pins are assigned in one place, `firmware/turret/src/board.rs`; power, wiring and parts are in [wiring.md](wiring.md). Timings, pulse ranges, gimbal limits and battery constants live in `TurretConfig` (`crates/turret-core/src/config.rs`).

### Firmware structure

Hardware-independent logic is in `crates/turret-core` and unit-tested on the host: the SCS protocol, pulse/duty maths, battery maths, the command parser and the fire sequencer. The firmware runs one 5 ms control loop that owns all actuators. A console thread reads lines from USB-Serial-JTAG and the NimBLE host task decodes Command writes; both send commands over one channel. The loop publishes state snapshots and fire events to a `ble` thread, which rate-limits and notifies them, so no BLE call runs in the control loop. A BLE disconnect disarms. The light gate ISR only stores timestamps in atomics.

The fire sequencer is a non-blocking state machine: spin up → push → retract → (next shot) → linear spin-down → cooldown. It refuses to fire while disarmed, busy, cooling down, or with zero speed. Bursts and auto fire are capped at `max_shots`. Disarm cuts the throttle and retracts the pusher immediately.

### Development console

Open with `espflash monitor` (or `cargo run`). Typed characters are echoed; Enter sends the line.

| Command | Effect |
|---|---|
| `help` | List commands |
| `arm` / `disarm` | Allow / stop flywheels and pusher (boots disarmed) |
| `status` | Arming, fire phase, battery, last gimbal target |
| `ping <id>` | Ping a serial bus servo |
| `P;pan;tilt;vpan;vtilt` | Move the gimbal, servo units |
| `F;1;mode;throttle;burst` | Fire; mode 0 single, 1 burst, 2 auto; throttle 0–100 % |
| `F;0` | Cease fire after the current shot |
| `B` | Battery voltage and percent |
| `unpair` | Forget all bonded phones |

### Vision node (`firmware/vision`, not created yet)

An ESP32-S3 camera board with PSRAM; the planned board is the Freenove ESP32-S3 WROOM CAM (OV2640 with hardware JPEG, 8 MB PSRAM). Face detection and recognition use Espressif's esp-dl / ESP-WHO models, linked into the Rust app as an ESP-IDF component through FFI. Output per frame: target box in normalized image coordinates, authorized/unauthorized, and confidence. Enrolled face embeddings stay on the device.

### Inter-node link (*open*)

Default: ESP-NOW with `turret-proto` messages encoded by postcard. That needs no cable through the gimbal and has low latency without a router. If the camera ends up wired next to the controller, a UART link carries the same messages instead.

### Video stream to the phone (*open*)

The phone app shows the camera view with a crosshair for manual targeting. Video does not go over BLE: Web Bluetooth moves roughly 20–100 KB/s, a QVGA JPEG is 8–15 KB, and the frames would compete with Aim and Fire writes. The video also originates on the vision node, which has no BLE link to the phone.

- BLE stays the only control channel. The video link is display-only and carries no commands.
- The vision node runs a Wi-Fi softAP and serves frames from the OV2640's hardware JPEG, as MJPEG over HTTP or JPEG frames over a WebSocket. The phone joins the AP.
- The softAP and ESP-NOW share the radio and must use the same channel.
- Streaming and face detection share the CPU. Detection runs at about 5–10 fps on QVGA, so the stream may be limited to the frames that went through detection.
- With the camera on the gimbal, the crosshair is a fixed overlay at the boresight point, calibrated once for the camera-to-barrel offset. Tap-to-aim would need a calibration from image coordinates to pan/tilt; that maths goes in `turret-core`.
- The vision node can send its face boxes and classifications alongside the frames so the app draws them over the video.

Open questions:
- **Mixed content.** The PWA is served over HTTPS (Web Bluetooth needs a secure context), and the camera serves plain HTTP on a private address, which Chrome blocks from an HTTPS page. Candidates: Chrome Local Network Access (`fetch(url, { targetAddressSpace: 'local' })` after a permission prompt, drawn to a `<canvas>`; check support on Android Chrome first), or HTTPS on the vision node with a self-signed certificate accepted once per phone (costs CPU next to detection). Serving the PWA from the ESP does not work, because a plain HTTP origin is not a secure context.
- **Faces in manual aim.** A camera view makes it easy to put the crosshair on a face. Decide whether manual aiming from the app also gets the torso offset or a face lockout from the vision node's boxes before the stream ships.

## Targeting loop

1. The vision node reports a face box and its classification.
2. The aim point is placed **below** the face box (torso). The turret never aims at a face.
3. The pixel error from the image center drives the pan/tilt controller.
4. The TFmini Plus range gives elevation compensation from an empirically measured table.
5. All safety gates pass → spin up flywheels → push one dart → measure speed → ramp down.

## Safety requirements

- A physical arm switch must be on to spin flywheels; BLE disarm and kill work at any time. The switch also cuts the pusher's power in hardware.
- No fire on uncertainty: unknown, low-confidence or authorized faces are never engaged.
- Torso aim offset; no fire when the aim point cannot be placed below the face.
- Minimum engagement range from the lidar; no fire without a valid range reading.
- Rate limit between shots, and a timeout that stops the flywheels.
- Firing logic never blocks the control loop (the original did).

## BLE protocol

The turret is a BLE peripheral for our own [phone app](#phone-app-app). This section is the contract between the two. The firmware side is `turret_core::remote` (codec, host-tested) and `firmware/turret/src/ble.rs` (NimBLE); the app side is `app/src/app/protocol/turret-protocol.ts`.

- Advertised name `DART Turret`. The 128-bit service UUID is not advertised (it does not fit next to the name), so Web Bluetooth filters on `namePrefix: "DART"` and lists the service in `optionalServices`.
- Service `8d6f0001-3b2a-4c1e-9f57-2e1d0c7a5b40`; the characteristics share the UUID with `0001` replaced.
- Every characteristic requires an authenticated link: LE Secure Connections with a static 6-digit passkey, compiled in from `TURRET_BLE_PASSKEY`. The first access triggers OS pairing; bonds are kept in NVS until `unpair` on the console.
- One connection at a time. Losing it disarms the turret.
- Little-endian, fixed layouts, every payload ≤ 20 bytes so the default ATT MTU suffices.

| Characteristic | UUID | Properties | Payload |
|---|---|---|---|
| Command | `8d6f0002-…` | Write with response | `op u8` + arguments |
| State | `8d6f0003-…` | Read, Notify | 10 bytes, below |
| Event | `8d6f0004-…` | Notify | `kind u8` + arguments |

Commands. A write that does not decode is answered with ATT error `0x80` (unknown op) or `0x81` (bad length or value); otherwise the outcome arrives as State and Event notifications.

| Op | Command | Arguments |
|---|---|---|
| `0x01` | Arm | – |
| `0x02` | Disarm | – |
| `0x03` | Aim | `pan u16, tilt u16, pan_speed u16, tilt_speed u16`, servo units, clamped to the gimbal limits |
| `0x04` | Fire | `mode u8` (0 single, 1 burst, 2 auto), `throttle u8` (0–100 %, 0 is rejected), `shots u8` (burst only, capped at `max_shots`) |
| `0x05` | Cease fire | – |

State, notified on change at most every 50 ms, once when notifications are turned on, and readable at any time:

| Byte | Field |
|---|---|
| 0 | Protocol version, currently 1; the app refuses versions it does not know |
| 1 | Flags: bit 0 armed, bit 1 fire sequencer idle and cooled down |
| 2 | Fire phase: 0 idle, 1 spin-up, 2 pushing, 3 retracting, 4 spin-down |
| 3–4 | Battery pack mV, 0 until the first reading |
| 5 | Battery percent |
| 6–7, 8–9 | Last commanded pan, tilt; `0xFFFF` before the first move |

Events, sent whatever started the action (console or app); a pending State change is flushed first so the two stay in order:

| Kind | Event | Arguments |
|---|---|---|
| `0x01` | Shot | `number u8, speed_kmh u16` (`0xFFFF` without a light gate reading) |
| `0x02` | Sequence finished | `shots u8` |
| `0x03` | Sequence aborted (disarm or link loss) | – |
| `0x04` | Fire rejected | `reason u8`: 0 disarmed, 1 busy, 2 cooling down, 3 no throttle |
| `0x05` | Gimbal move failed | – |

## Phone app (`app/`)

Angular 22 PWA over Web Bluetooth, for Chromium on Android and desktop. It is served from GitHub Pages and can be installed to the home screen. It is zoneless and signal-based: Bluetooth notifications write straight into signals, and commands are only sent from user input.

| Path (`app/src/app/`) | Role |
|---|---|
| `protocol/turret-protocol.ts` | The [BLE protocol](#ble-protocol) codec, the TypeScript twin of `turret_core::remote`. Its tests use the same byte vectors as the Rust tests |
| `link/` | `TurretLink` interface and the Web Bluetooth implementation. `CommandQueue` keeps one write in flight: a newer Aim replaces a queued one, Disarm drops everything queued, and Cease fire jumps ahead of queued Fire |
| `turret/turret.ts` | Connection, state, event log and shots as signals; the only place commands are sent from |
| `aim/aim-controller.ts` | Rate-control joystick maths: deadzone, expo, integration, clamp to 0–1023, at most one Aim per 50 ms. Tunables are in `AIM_TUNING` |
| `ui/` | Connect screen, control panel, joystick, hold-to-arm switch, fire panel (Signal Forms), telemetry |

The app adds these safety behaviours on top of the firmware's rules, without replacing them:
- Arming needs a 1 s hold.
- Disarm is one tap and always visible while armed.
- The page disarms the turret when it is hidden.
- Auto fire only runs while the button is held.
- Fire is only enabled when the turret reports armed and ready.
- The app refuses a turret with an unknown protocol version.

Develop with `npm start` in `app/` and open `http://localhost:4200` in Windows Chrome; `localhost` counts as a secure context. To use a phone, forward port 4200 from `chrome://inspect`, or use the Pages build. `.github/workflows/app.yml` checks formatting, runs the tests, builds with the repository name as base href, and deploys `main` to Pages.

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
   b. BLE peripheral with our own protocol (see [BLE protocol](#ble-protocol)), passkey bonding, disarm on link loss. *(done, tested on hardware)*
   c. Angular PWA in `app/` over Web Bluetooth: pair, arm/disarm, joystick aim, fire modes, state and shot telemetry, installable. Runs in Chromium browsers on Android and desktop; iOS Safari has no Web Bluetooth. *(done, tested against a simulated GATT server; pairing and aim tuning still to be checked on the turret)*
3. TFmini Plus driver and range-based elevation table.
4. a. Vision node: camera, esp-dl face detection and recognition, enrollment.
   b. Video stream to the phone app with a crosshair overlay (see [Video stream to the phone](#video-stream-to-the-phone-open)); settle mixed content and face handling in manual aim first.
5. ESP-NOW link, autonomous mode and the safety gates above.
