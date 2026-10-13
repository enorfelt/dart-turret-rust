# dart-turret-rust

ESP32-S3 Nerf dart turret firmware in Rust on ESP-IDF. Design, pin map, safety rules and roadmap: `docs/architecture.md`.

## Layout

- `crates/*` — host workspace (stable toolchain). `turret-proto` is `no_std` and shared by host and firmware.
- `firmware/<node>` — one ESP-IDF app per node, outside the workspace, with its own `Cargo.lock`, `rust-toolchain.toml` (`esp`) and `.cargo/config.toml` (target, `build-std`, ESP-IDF version).

## Commands

Run firmware commands from the firmware crate directory. They need the Xtensa env from `~/export-esp.sh`; prefix with `. ~/export-esp.sh &&` if `cargo build` cannot find `xtensa-esp-elf` or libclang.

```sh
cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check   # repo root
cd firmware/turret && cargo build && cargo clippy -- -D warnings               # firmware
cd firmware/turret && cargo run                                                # flash + monitor, needs usbipd attach
```

Flashing from Windows without USB passthrough: `pwsh scripts/flash.ps1 -Port COMx`.

The first firmware build downloads ESP-IDF into `~/.espressif` and takes several minutes.

## Conventions

- No code comments by default. Only write one when the code looks wrong but is not, or is genuinely hard to follow, and then explain why in one or two lines.
- Keep hardware-independent logic (protocols, parsing, control math, ballistics) in host-testable crates under `crates/` and unit-test it there.
- Never weaken the safety requirements in `docs/architecture.md` without being asked to.
