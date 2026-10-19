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
| Pusher | 41 | RC servo, 50 Hz via LEDC, 5 V signal through the level shifter | Retracted 500 µs, extended 833 µs: the original's 333 µs (about 30°) of travel, reversed to suit the printed pusher |
| Range | RX 6, TX 7 | TFmini Plus on UART2, 115200 8N1 | 9-byte frames `0x59 0x59 distL distH strL strH tempL tempH checksum`, 0.1–12 m (step 3) |
| Dart speed | 42 | IR light gate, any-edge GPIO interrupt | Time between the two edges over a 70 mm dart gives km/h |
| Battery | 1 | 4S Li-ion through a 39k/8k2 divider to ADC1_CH0 | 16.8 V reads ≈ 2.92 V. ADC1, so readings keep working with Wi-Fi and ESP-NOW on. Read every 5 s while the flywheels are idle; below 3.3 V per cell, or below 10 V (a reversed or dead cell), `arm` is refused and an armed turret disarms. Below 5 V counts as no pack (USB power) and does not block arming |
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
| `arm` / `disarm` | Allow / stop flywheels and pusher (boots disarmed); `arm` needs the arm switch on and a battery that is not empty or faulty |
| `status` | Arming, fire phase, battery, last gimbal target |
| `ping <id>` | Ping a serial bus servo |
| `pusher <µs>` / `pusher off` | Hold the pusher at a pulse of 500–2500 µs to calibrate it, or release it to retracted. Disarmed only; `arm` and losing the BLE link release it. The servo only has power with the arm switch on |
| `P;pan;tilt;vpan;vtilt` | Move the gimbal, servo units |
| `F;1;mode;throttle;burst` | Fire; mode 0 single, 1 burst, 2 auto; throttle 0–100 % |
| `F;0` | Cease fire after the current shot |
| `B` | Battery voltage, percent and health |
| `unpair` | Forget all bonded phones |

### Vision node (`firmware/vision`)

A Freenove ESP32-S3 WROOM CAM (8 MB octal PSRAM, 8 MB flash) with a microSD card. Its camera is a GC0308, a VGA sensor without hardware JPEG; an OV2640 on the same connector works unchanged; pins and power are in [wiring.md](wiring.md#vision-node). Face detection (`espressif/human_face_detect`, MSR+MNP) and face features (`espressif/human_face_recognition`, MFN) are esp-dl components fetched by the ESP-IDF component manager. A small C++ shim (`firmware/vision/components/face`) gives them a C API that is bound into Rust. The models are compiled into flash, so detection does not need the card.

Each QVGA frame goes: RGB565 capture → software JPEG encode for the stream → detect on the raw pixels → IoU tracker → at most one face feature → identity → stream. `status` on the vision console prints the frame rate and the time of each stage. The hardware-independent parts are in `crates/vision-core` and unit-tested on the host: tracker, gallery matching, identity policy, recognition schedule, enrollment rules, people file and stream codec. Thresholds are in `VisionConfig`.

Recognition:
- Every enrolled person is either *protected* or a *target*. Unknown faces are never engaged (see [Safety requirements](#safety-requirements)), so only people enrolled as targets can ever be engaged: being a target is opt-in.
- A face matches a person by cosine similarity of the L2-normalised features against each of the person's samples (`match_similarity`), with a clear margin over the next person (`match_margin`).
- A track becomes *target* only after `target_confirmations` agreeing recognitions of the same target person. One failed recheck drops it back to unknown. A track that ever matches a protected person, as best match or runner-up, stays protected until the track is lost.
- A face feature costs about 250 ms, so detection runs on every frame and recognition runs per track: new and unconfirmed tracks first, then confirmed targets every `recheck_ms`.
- Enrollment needs exactly one face in view and takes `enroll_samples` samples. It is refused when the face already matches someone, so a protected person cannot be enrolled again as a target under another name. An extra sample must match the person it is added to.
- People are stored on the card as `people.bin` (postcard with a format version and the feature length, so a model change is detected) plus one enrollment snapshot each in `people/<id>.jpg`. Without a card, detection and the stream still run, every face is unknown, and enrollment is off.

Camera choice:
- **Range is set by the lens and the models, not the sensor.** The detector scales each frame down to 160×120 and recognition works on 112×112 face crops, so QVGA is enough and a higher resolution buys little. With a lens of about 60° across, a 15 cm face is roughly 30 px wide at 1.5 m and 15 px at 3 m, so expect reliable detection and recognition to about 1.5–2.5 m. A narrower lens (40–50°) makes faces larger and extends that range, at the cost of a smaller view that the gimbal partly makes up for. A wide-angle lens shortens it.
- **GC0308 drawbacks** (the board's stock sensor): no hardware JPEG, so the S3 encodes every frame on the same core as detection and recognition; a noisier image with weaker colour and exposure in poor indoor light, which hurts both models; and fewer exposure and white-balance controls in esp32-camera. It is fine for bring-up.
- **Alternatives on the same 24-pin connector, all supported by esp32-camera:** the OV2640 has a much better image, hardware JPEG and many lens options, and is cheap and common. The OV3660 is better again in low light. The OV5640 has the best image and autofocus versions, but more power and heat, mostly wasted at QVGA.
- **Plan:** keep the GC0308 until the node works on hardware, then move to an OV2640 with a normal or narrow lens. With it, capture the sensor's hardware JPEG and decode it in software for detection, so the stream needs no encode. Choose the pipeline from the detected sensor in `camera.rs` and the face shim, rather than hard-coding one.

Network:
- WPA2 softAP `DART Vision` on a fixed channel 1, password compiled in from `VISION_WIFI_PASSWORD` (8–63 characters). The channel is fixed because ESP-NOW (step 5) shares the radio. The password is the only access control for enrollment.
- `http://192.168.71.1/` serves the enrollment page: the live view with face boxes, enroll by name and role, add samples, remove people.
- `ws://192.168.71.1/stream` carries the [vision stream](#vision-stream-protocol), at most 3 clients.
- JSON API used by the page: `GET /api/people`, `POST /api/people?name=…&role=target|protected`, `POST /api/people/{id}/samples`, `DELETE /api/people/{id}`, `GET /api/people/{id}/photo`, `GET /api/enrollment` (idle, running, done with the id, or failed with a reason). Enrollment runs on the vision thread; the page polls its status.
- The vision console is on UART0, the board's CH343 USB port, with the log mirrored to the native USB port. It has `help`, `status` and `people`. A startup failure is logged and the node stops instead of rebooting in a loop.

### Inter-node link (*open*)

Default: ESP-NOW with `turret-proto` messages encoded by postcard. That needs no cable through the gimbal and has low latency without a router. If the camera ends up wired next to the controller, a UART link carries the same messages instead.

### Video stream to the phone

The phone app shows the camera view with a crosshair for manual targeting. Video does not go over BLE: Web Bluetooth moves roughly 20–100 KB/s, a QVGA JPEG is 8–15 KB, and the frames would compete with Aim and Fire writes. The video also originates on the vision node, which has no BLE link to the phone.

- BLE stays the only control channel. The video link is display-only and carries no commands.
- The phone joins the vision node's softAP and opens `ws://192.168.71.1/stream`. Each message is one [vision stream](#vision-stream-protocol) frame: a software-encoded JPEG plus the face boxes and classes from the same frame. Only frames that went through detection are sent, so the stream rate is the detection rate.
- The softAP and ESP-NOW share the radio and must use the same channel.
- **Mixed content** is handled with Chrome Local Network Access. The PWA is served over HTTPS and the camera over plain HTTP; the app opens the WebSocket with `{ targetAddressSpace: 'local' }` (Chrome 154+), and a private IP literal counts as local. Chrome asks once for the local network permission. This works on Android Chrome with mobile data on and BLE to the turret connected at the same time. Chrome then marks the page as not secure in the address bar, because it loads a plain `ws://` stream. If the stream ever fails to connect, check that the phone stayed on the AP; Android may offer to leave a network without internet. The fallback, if LNA ever stops working, is a `fetch()`-streamed multipart MJPEG, which LNA also allows.
- **Faces in manual aim.** The app draws each face box in its class colour and a torso hint below it (an arrow at the bottom edge when the torso is out of frame), but never blocks Fire: in manual mode the operator is responsible. Autonomous mode (step 5) places the aim point below the face on the controller.
- The overlay has a fixed crosshair at the image centre (camera boresight). The floating impact reticle, which shows where the next dart is expected to land, comes after step 3 (see [Sensor mounting and aim point](#sensor-mounting-and-aim-point-open)). Tap-to-aim would need a calibration from image coordinates to pan/tilt; that maths goes in `turret-core`.

## Sensor mounting and aim point (*open*)

The camera and the TFmini Plus are mounted on the tilt stage, so they move with the barrel, with their axes parallel to the barrel rather than converged on it.

```
        [ camera ]      directly above the barrel, h ≈ 3–5 cm
        [ TFmini ]      next to the camera
  ══════(  barrel  )══════►
```

- **Camera directly above the barrel.** With no lateral offset, parallax and drop are both vertical, so the impact point moves along one image column and the calibration has one dimension. Any remaining lateral error is a constant trim.
- **Parallel, not converged.** The camera-to-barrel offset `h` stays the same in metres at every range, so its angle is `atan(h / R)`. A converged mount is only right at one range.
- **Lidar next to the camera.** The range must come from what is under the crosshair. The TFmini Plus beam spreads a few degrees (check the datasheet), roughly 15–35 cm across at 5 m, so a small target at range partly returns the background. The app can draw the beam footprint so an unreliable reading is visible.
- **Mass and vibration.** Keep the sensors close to the tilt axis, and soft-mount the camera against flywheel vibration. The camera talks over ESP-NOW and only needs power through the gimbal; the lidar's UART runs through the gimbal to the controller.

The impact point in the image, with the camera's vertical focal length `fy` in pixels and the dart's drop `drop(R, v)` at range `R` and dart speed `v`:

```
pixel_y = cy + fy · (h + drop(R, v)) / R
```

With h = 4 cm and the stock OV2640 lens at QVGA (fy ≈ 250–300 px), parallax alone is about 10 px at 1 m and 2 px at 5 m.

- **Drop is measured, not modelled.** Drag, wobble and flywheel spin dominate foam-dart flight, and darts can rise at short range. Shoot groups at a board at about 1, 2, 3, 5 and 8 m and two or three throttles, record the mean impact in image pixels, and interpolate. Recording in pixels makes the focal length unnecessary.
- **Indexed by dart speed, not throttle.** ESC throttle is open loop and drifts with pack voltage. The table is indexed by light gate speed, and the next shot's speed is estimated from recent shots at the same throttle (or from throttle and pack voltage until there are any).
- **Spread.** The reticle is a circle that grows with range from the measured group size, not a point.

The table and the interpolation live in `turret-core` and are unit-tested on the host. The same function serves every mode:
- **Manual:** the app draws the floating reticle from the offset the controller reports.
- **Assisted manual:** the user holds the fixed crosshair on the target and the controller adds the tilt offset before firing.
- **Autonomous:** the controller adds the tilt offset so the impact point lands on the torso aim point.

The app needs the range and the reticle offset, so State gets new fields in protocol version 3: range in cm with a valid flag, and the reticle offset and radius computed on the controller, so the table lives in one place. Video arrives over Wi-Fi and range over BLE with different delays; that is acceptable for the overlay, but the no-fire-without-range rule always uses the controller's own fresh reading.

## Targeting loop

1. The vision node reports a face box and its classification.
2. The aim point is placed **below** the face box (torso). The turret never aims at a face.
3. The pixel error from the image center drives the pan/tilt controller.
4. The TFmini Plus range and the expected dart speed give the elevation offset from the impact table (see [Sensor mounting and aim point](#sensor-mounting-and-aim-point-open)).
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
| State | `8d6f0003-…` | Read, Notify | 12 bytes, below |
| Event | `8d6f0004-…` | Notify | `kind u8` + arguments |

Commands. A write that does not decode is answered with ATT error `0x80` (unknown op) or `0x81` (bad length or value); otherwise the outcome arrives as State and Event notifications.

| Op | Command | Arguments |
|---|---|---|
| `0x01` | Arm | – |
| `0x02` | Disarm | – |
| `0x03` | Aim | `pan u16, tilt u16, pan_speed u16, tilt_speed u16`, servo units, clamped to the gimbal limits |
| `0x04` | Fire | `mode u8` (0 single, 1 burst, 2 auto), `throttle u8` (0–100 %, 0 is rejected), `shots u8` (burst only, capped at `max_shots`) |
| `0x05` | Cease fire | – |
| `0x06` | Pusher calibration | `pulse_us u16`: 500–2500 holds the pusher at that pulse, 0 releases it to retracted. Ignored while armed; `arm` and losing the link release it |

State, notified on change at most every 50 ms, once when notifications are turned on, and readable at any time:

| Byte | Field |
|---|---|
| 0 | Protocol version, currently 3; the app refuses versions it does not know |
| 1 | Flags: bit 0 armed, bit 1 fire sequencer idle and cooled down, bit 2 physical arm switch on |
| 2 | Fire phase: 0 idle, 1 spin-up, 2 pushing, 3 retracting, 4 spin-down |
| 3–4 | Battery pack mV, 0 until the first reading |
| 5 | Battery percent |
| 6–7, 8–9 | Last commanded pan, tilt; `0xFFFF` before the first move |
| 10–11 | Pulse the pusher is held at for calibration, µs; 0 when not held |

Events, sent whatever started the action (console or app); a pending State change is flushed first so the two stay in order:

| Kind | Event | Arguments |
|---|---|---|
| `0x01` | Shot | `number u8, speed_kmh u16` (`0xFFFF` without a light gate reading) |
| `0x02` | Sequence finished | `shots u8` |
| `0x03` | Sequence aborted (disarm or link loss) | – |
| `0x04` | Fire rejected | `reason u8`: 0 disarmed, 1 busy, 2 cooling down, 3 no throttle |
| `0x05` | Gimbal move failed | – |

## Vision stream protocol

The contract between the vision node and the phone app (and the enrollment page). After the WebSocket opens, the client sends one message of any content to subscribe: ESP-IDF 5.5 answers the handshake without calling the server's handler, so the node only learns about a client from its first message. The node then sends frames only while at least one client is subscribed. Each WebSocket binary message from the node is one frame, little-endian. The codec is `vision_core::stream` on the node and `app/src/app/protocol/vision-stream.ts` in the app, with the same byte vectors in both test suites.

| Byte | Field |
|---|---|
| 0 | Stream version, currently 1; the app refuses versions it does not know |
| 1 | Flags: bit 0 recognition on (card mounted and someone enrolled), bit 1 enrollment running |
| 2–3 | Frame sequence number, wrapping |
| 4–5, 6–7 | Image width, height in pixels |
| 8 | Face count `n`, at most 8 |
| 9 … | `n` faces of 11 bytes: `track u8`, `class u8` (0 unknown, 1 protected, 2 target), `confidence u8` (similarity in %, 0 when unknown), `x, y, w, h u16` in image pixels |
| 9 + 11·n … | The JPEG image |

## Phone app (`app/`)

Angular 22 PWA over Web Bluetooth, for Chromium on Android and desktop. It is served from GitHub Pages and can be installed to the home screen. It is zoneless and signal-based: Bluetooth notifications write straight into signals, and commands are only sent from user input.

| Path (`app/src/app/`) | Role |
|---|---|
| `protocol/turret-protocol.ts` | The [BLE protocol](#ble-protocol) codec, the TypeScript twin of `turret_core::remote`. Its tests use the same byte vectors as the Rust tests |
| `link/` | `TurretLink` interface and the Web Bluetooth implementation. `CommandQueue` keeps one write in flight: a newer Aim or Pusher command replaces a queued one, Disarm drops everything queued, and Cease fire jumps ahead of queued Fire |
| `turret/turret.ts` | Connection, state, event log and shots as signals; the only place commands are sent from |
| `aim/aim-controller.ts` | Rate-control joystick maths: deadzone, expo, integration, clamp to 0–1023, at most one Aim per 50 ms. Tunables are in `AIM_TUNING` |
| `protocol/vision-stream.ts` | The [vision stream](#vision-stream-protocol) codec, the TypeScript twin of `vision_core::stream` |
| `vision/` | `VisionLink` (WebSocket with Local Network Access) and the `Vision` service: connection, latest frame, retry, camera address kept in localStorage. `overlay.ts` holds the torso hint geometry and `OVERLAY_TUNING` |
| `ui/` | Connect screen, control panel, joystick, hold-to-arm switch, fire panel (Signal Forms), telemetry, pusher calibration (shown while disarmed and idle, without the camera), video view (camera canvas with crosshair, face boxes and torso hints; greyed out when a frame is over 500 ms old). With the camera on, the page goes full screen in landscape and the video fills it, with the status line, arm switch, joystick and a compact fire panel overlaid and the log hidden |

The app adds these safety behaviours on top of the firmware's rules, without replacing them:
- Arming needs a 1 s hold.
- Disarm is one tap and always visible while armed.
- The page disarms the turret when it is hidden, and releases a held pusher.
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
3. TFmini Plus driver, the impact table indexed by range and dart speed, and range and reticle offset in State (protocol version 3).
4. a. Vision node: camera on the tilt stage, esp-dl face detection and recognition, enrollment from a web page on its softAP, people on the SD card. *(done, tested on the board with the GC0308)*
   b. Video stream to the phone app with the fixed crosshair, face boxes and torso hints (see [Video stream to the phone](#video-stream-to-the-phone)). *(done, tested on an Android phone)* After step 3: the floating impact reticle, and calibrating the impact table by shooting groups through the camera.
5. ESP-NOW link, autonomous mode and the safety gates above.
