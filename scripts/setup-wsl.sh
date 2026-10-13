#!/usr/bin/env bash
set -euo pipefail

APT_PACKAGES=(
  git curl wget build-essential pkg-config libudev-dev libssl-dev libffi-dev
  python3 python3-venv python3-pip cmake ninja-build ccache flex bison gperf
  dfu-util libusb-1.0-0 usbutils
)
CARGO_TOOLS=(espup ldproxy espflash cargo-espflash)
ESP_TARGETS="esp32s3,esp32"
EXPORT_FILE="$HOME/export-esp.sh"
BASHRC_MARKER="# dart-turret: esp toolchain env"

update=false
[[ "${1:-}" == "--update" ]] && update=true

step() { printf '\n==> %s\n' "$*"; }

is_native_command() {
  local path
  path="$(command -v "$1" 2>/dev/null)" || return 1
  [[ "$path" != /mnt/* ]]
}

install_apt_packages() {
  local missing=()
  for package in "${APT_PACKAGES[@]}"; do
    dpkg -s "$package" >/dev/null 2>&1 || missing+=("$package")
  done
  if ((${#missing[@]})); then
    step "Installing apt packages: ${missing[*]}"
    sudo apt-get update
    sudo apt-get install -y "${missing[@]}"
  fi
}

install_rustup() {
  if ! is_native_command rustup; then
    step "Installing rustup"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  fi
  source "$HOME/.cargo/env"
}

install_cargo_tools() {
  for tool in "${CARGO_TOOLS[@]}"; do
    if $update || ! is_native_command "$tool"; then
      step "Installing $tool"
      cargo install --locked "$tool"
    fi
  done
}

install_esp_toolchain() {
  if ! rustup toolchain list | grep -q '^esp'; then
    step "Installing Xtensa Rust toolchain ($ESP_TARGETS)"
    espup install --targets "$ESP_TARGETS" --export-file "$EXPORT_FILE"
  elif $update; then
    step "Updating Xtensa Rust toolchain"
    espup update --export-file "$EXPORT_FILE"
  fi
}

source_export_file_from_bashrc() {
  if ! grep -qF "$BASHRC_MARKER" "$HOME/.bashrc"; then
    step "Sourcing $EXPORT_FILE at the top of ~/.bashrc (before the interactive guard)"
    {
      echo "$BASHRC_MARKER"
      echo "[ -f \"$EXPORT_FILE\" ] && . \"$EXPORT_FILE\""
      echo
      cat "$HOME/.bashrc"
    } >"$HOME/.bashrc.dart-turret.tmp"
    mv "$HOME/.bashrc.dart-turret.tmp" "$HOME/.bashrc"
  fi
}

install_claude_code() {
  if ! is_native_command claude; then
    step "Installing Claude Code"
    curl -fsSL https://claude.ai/install.sh | bash
  elif $update; then
    step "Updating Claude Code"
    claude update
  fi
}

install_apt_packages
install_rustup
install_cargo_tools
install_esp_toolchain
source_export_file_from_bashrc
install_claude_code

step "Done. Open a new shell, then: cd firmware/turret && cargo build"
