#!/usr/bin/env bash
# Run Tauri dev with Wayland backend (for Hyprland/Plasma-Wayland sessions
# where WAYLAND_DISPLAY isn't propagated to the shell).
set -euo pipefail

# Kill any leftover Vite/dev server on 5173 so we don't fail on restart.
fuser -k 5173/tcp 2>/dev/null || true

export PATH="/home/oliverm/.cargo/bin:$PATH"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export XDG_SESSION_TYPE=wayland
export GDK_BACKEND=wayland
export WEBKIT_DISABLE_DMABUS_RENDERER=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1
export LIBGL_ALWAYS_SOFTWARE=1
export _JAVA_AWT_WM_NONREPARENTING=1
cd "$(dirname "$0")"
cargo-tauri dev "$@"