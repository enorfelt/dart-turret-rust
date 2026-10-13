# dart-turret-rust

Rust firmware on ESP-IDF for the [Dart-Turret](https://www.printables.com/model/338574) Nerf turret (ESP32-S3), extended with autonomous targeting: an ESP32-S3 camera node for face detection and recognition, and a TFmini Plus lidar for range. See [docs/architecture.md](docs/architecture.md).

## Layout

```
crates/turret-proto   no_std messages shared by all nodes (serde + postcard), host-tested
firmware/turret       ESP32-S3 turret controller (esp-idf-svc)
scripts/              WSL toolchain setup, Windows flashing helper
docs/                 architecture, safety rules, roadmap
```

## Setup (Windows + WSL Ubuntu)

Keep the clone in the WSL filesystem (e.g. `~/src/dart-turret-rust`); ESP-IDF builds on `/mnt/c` are very slow.

```sh
./scripts/setup-wsl.sh            # apt packages, rustup, espup (esp32s3 + esp32), espflash, ldproxy, Claude Code
./scripts/setup-wsl.sh --update   # update the same tools later
```

Open a new shell afterwards so `~/export-esp.sh` is sourced. Open the folder from VS Code or Cursor with the WSL remote extension so rust-analyzer runs inside WSL.

## Build

```sh
cargo test                                   # host crates, repo root
cd firmware/turret && cargo build            # first build downloads ESP-IDF v5.5 into ~/.espressif
```

## Flash and monitor

From Windows, no USB passthrough needed (espflash 4.x recommended: `cargo install espflash --locked`):

```powershell
pwsh \\wsl.localhost\Ubuntu\home\<user>\src\dart-turret-rust\scripts\flash.ps1 -Port COM5
```

From WSL, after attaching the board with [usbipd-win](https://github.com/dorssel/usbipd-win) in an admin PowerShell:

```powershell
usbipd list
usbipd bind --busid <BUSID>
usbipd attach --wsl --busid <BUSID>
```

```sh
cd firmware/turret && cargo run
```

## Claude Code

`setup-wsl.sh` installs Claude Code natively in WSL. Run `claude` in the repo root and log in once. Project instructions are in `CLAUDE.md`; shared permissions in `.claude/settings.json`.
