# dart-turret-rust

Rust firmware on ESP-IDF for the [Dart-Turret](https://www.printables.com/model/338574) Nerf turret (ESP32-S3), extended with autonomous targeting: an ESP32-S3 camera node for face detection and recognition, and a TFmini Plus lidar for range. See [docs/architecture.md](docs/architecture.md).

## Layout

```
crates/turret-core    no_std turret logic: SCS protocol, fire sequencer, command parser, BLE protocol codec
crates/turret-proto   no_std messages shared by all nodes (serde + postcard), host-tested
firmware/turret       ESP32-S3 turret controller (esp-idf-svc)
app/                  phone app: Angular PWA over Web Bluetooth
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

The firmware build needs the 6-digit BLE pairing passkey in `TURRET_BLE_PASSKEY`. It is compiled in and never committed. Put it in your shell profile, e.g. `echo 'export TURRET_BLE_PASSKEY=123456' >> ~/.bashrc` with your own digits. Changing it requires `unpair` on the console and pairing the phone again.

## Phone app

The app in `app/` controls the turret from Chrome or Edge on Android or a desktop computer. Safari and Firefox have no Web Bluetooth.

The app uses Node 24 from nvm in WSL:

```sh
nvm install 24          # once
cd app && npm ci
npm start               # http://localhost:4200, open it in Windows Chrome
npm test -- --watch=false
```

Pushes to `main` that touch `app/` are deployed to GitHub Pages by `.github/workflows/app.yml`. This needs Pages enabled with "GitHub Actions" as the source. Open the Pages URL on the phone, connect, and enter the passkey when Android asks. Use "Add to Home screen" to install the app.

## Flash and monitor

### From Windows (recommended)

No USB passthrough needed. Requires espflash 4.x on Windows: `cargo +stable install espflash --locked`.

```powershell
pwsh -ExecutionPolicy Bypass -File \\wsl.localhost\Ubuntu\home\<user>\src\dart-turret-rust\scripts\flash.ps1 -Port COM4
```

`-ExecutionPolicy Bypass` is needed because `RemoteSigned` treats `\\wsl.localhost` paths as remote. The script only wraps `espflash flash --monitor --port COM4 <elf>`.

### From WSL

Attach the board with [usbipd-win](https://github.com/dorssel/usbipd-win) from Windows PowerShell (`bind` needs admin, once). `--auto-attach` matters on the ESP32-S3: its native USB re-enumerates when espflash resets it.

```powershell
usbipd list
usbipd bind --busid <BUSID>
usbipd attach --wsl --busid <BUSID> --auto-attach
```

```sh
cd firmware/turret && cargo run
```

If attach fails with `tcp connect` / "firewall appears to be blocking":

- Podman user-mode networking (`podman-net-usermode`) takes over the WSL default route, so usbipd connects to `192.168.127.1` instead of Windows. Pass the real host with `--host-ip <eth0 subnet>.1` (see `ip route` in WSL), or stop the Podman machine first.
- Windows Firewall must allow inbound TCP 3240 on the `vEthernet (WSL (Hyper-V firewall))` interface.

## Claude Code

`setup-wsl.sh` installs Claude Code natively in WSL. Run `claude` in the repo root and log in once. Project instructions are in `CLAUDE.md`; shared permissions in `.claude/settings.json`.
