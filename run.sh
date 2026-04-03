#!/usr/bin/env bash
set -euo pipefail

# A2RS launcher for Wayland desktops.
# Forces X11/XWayland so the window manager can provide a normal title bar.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
APP="${APP:-$SCRIPT_DIR/target/release/a2rs}"

if [[ ! -x "$APP" ]]; then
  echo "a2rs executable not found: $APP" >&2
  echo "Set APP=/path/to/a2rs or place this script next to the project tree." >&2
  exit 1
fi

# Prefer X11/XWayland for apps that do not handle Wayland decorations well.
export WINIT_UNIX_BACKEND=x11
export GDK_BACKEND=x11
export QT_QPA_PLATFORM=xcb
export SDL_VIDEODRIVER=x11
unset WAYLAND_DISPLAY

exec "$APP" "$@"
