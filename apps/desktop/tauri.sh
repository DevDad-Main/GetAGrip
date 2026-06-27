#!/usr/bin/env bash
# Run Tauri dev on X11 (works with NVIDIA on Cachyos; Wayland crashes GTK).
set -euo pipefail

# Kill any leftover Vite/dev server on 5173 so we don't fail on restart.
fuser -k 5173/tcp 2>/dev/null || true

export PATH="/home/oliverm/.cargo/bin:$PATH"
unset WAYLAND_DISPLAY
export GDK_BACKEND=x11
cd "$(dirname "$0")"
cargo-tauri dev "$@"
