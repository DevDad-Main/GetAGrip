#!/usr/bin/env bash
# Run Tauri dev with Wayland backend (for Hyprland/Plasma-Wayland sessions
# where WAYLAND_DISPLAY isn't propagated to the shell).
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export GDK_BACKEND=wayland
cd "$(dirname "$0")"
cargo tauri dev "$@"
