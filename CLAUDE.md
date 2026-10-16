# dart-turret-rust

ESP32-S3 Nerf dart turret firmware in Rust on ESP-IDF. Design, pin map, safety rules and roadmap: `docs/architecture.md`.

## Layout

- `crates/*` — host workspace (stable toolchain), all `no_std`. `turret-core` holds hardware-independent turret logic (SCS protocol, fire sequencer, command parser, BLE protocol codec in `remote.rs`, pulse and battery maths); `vision-core` holds the vision node's logic (tracker, recognition policy, people file, stream codec); `turret-proto` holds inter-node messages.
- The BLE protocol in `docs/architecture.md` is the contract with the phone app; change the spec, `turret_core::remote` and `app/src/app/protocol/turret-protocol.ts` together, with matching byte vectors in both test suites. The vision stream protocol works the same way with `vision_core::stream` and `app/src/app/protocol/vision-stream.ts`.
- `app/` — Angular 22 PWA (Web Bluetooth, zoneless, signals, Signal Forms, Vitest). Angular conventions are in `app/CLAUDE.md`. Run npm commands from `app/` inside WSL with nvm Node 24 (`app/.nvmrc`).
- `firmware/<node>/src/board.rs` is the single place GPIOs are assigned; tunables are in `turret_core::config::TurretConfig` and `vision_core::config::VisionConfig`.
- `firmware/<node>` — one ESP-IDF app per node (`turret`, `vision`), outside the workspace, with its own `Cargo.lock`, `rust-toolchain.toml` (`esp`) and `.cargo/config.toml` (target, `build-std`, ESP-IDF version).
- `firmware/vision/components/face` — C++ shim over Espressif's esp-dl face models, bound into Rust through `package.metadata.esp-idf-sys.extra_components`. Component versions are in its `idf_component.yml` and locked in `components_esp32s3.lock`.

## Commands

Run firmware commands from the firmware crate directory. They need the Xtensa env from `~/export-esp.sh`; prefix with `. ~/export-esp.sh &&` if `cargo build` cannot find `xtensa-esp-elf` or libclang. The turret firmware also needs `TURRET_BLE_PASSKEY` (6 digits) at build time; use `TURRET_BLE_PASSKEY=000000` for build checks when it is not set. The vision firmware needs `VISION_WIFI_PASSWORD` (8–63 characters); use `VISION_WIFI_PASSWORD=dart-vision-test` for build checks.

```sh
cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check   # repo root
cd firmware/turret && cargo build && cargo clippy -- -D warnings               # firmware
cd firmware/turret && cargo run                                                # flash + monitor, needs usbipd attach
cd firmware/vision && cargo build && cargo clippy -- -D warnings               # vision node, same toolchain
cd app && npm test -- --watch=false && npm run format:check && npm run build   # phone app
cd app && npm start                                                            # dev server on localhost:4200
```

Flashing from Windows without USB passthrough: `pwsh -ExecutionPolicy Bypass -File scripts/flash.ps1 -Port COMx` (add `-Firmware vision` for the vision node).

The first firmware build downloads ESP-IDF into `~/.espressif` and takes several minutes; the first vision build also downloads esp-dl and the camera driver.

## Conventions

- No code comments by default. Only write one when the code looks wrong but is not, or is genuinely hard to follow, and then explain why in one or two lines.
- Keep hardware-independent logic (protocols, parsing, control math, ballistics) in host-testable crates under `crates/` and unit-test it there.
- Never weaken the safety requirements in `docs/architecture.md` without being asked to.
