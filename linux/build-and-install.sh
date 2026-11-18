#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> Nook Enhanced Edition — Linux Installer"

if command -v apt-get >/dev/null 2>&1; then
    echo "==> Installing system dependencies (Debian/Ubuntu)..."
    sudo apt-get update
    sudo apt-get install -y \
        build-essential pkg-config python3 playerctl pulseaudio-utils \
        xclip wl-clipboard zenity tesseract-ocr ffmpeg \
        libx11-dev libxrandr-dev libxi-dev libxcursor-dev libxinerama-dev \
        libgl1-mesa-dev libegl1-mesa-dev libwayland-dev libxkbcommon-dev \
        libgtk-3-dev libayatana-appindicator3-dev || true
elif command -v dnf >/dev/null 2>&1; then
    echo "==> Installing system dependencies (Fedora)..."
    sudo dnf install -y \
        gcc pkgconfig python3 playerctl pulseaudio-utils \
        xclip wl-clipboard zenity tesseract ffmpeg \
        libX11-devel libXrandr-devel libXi-devel libXcursor-devel \
        mesa-libGL-devel wayland-devel libxkbcommon-devel \
        gtk3-devel libayatana-appindicator-gtk3-devel || true
elif command -v pacman >/dev/null 2>&1; then
    echo "==> Installing system dependencies (Arch)..."
    sudo pacman -S --needed --noconfirm \
        base-devel pkgconf python playerctl libpulse \
        xclip wl-clipboard zenity tesseract ffmpeg \
        libx11 libxrandr libxi libxcursor wayland libxkbcommon \
        gtk3 libayatana-appindicator || true
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "==> Rust not found. Installing rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi

echo "==> Building Nook (Release)..."
cd "$SRC_DIR"
if pkg-config --exists gtk+-3.0 ayatana-appindicator3-0.1 2>/dev/null; then
    cargo build --release --features linux-tray
else
    cargo build --release
fi

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
cp "$SRC_DIR/target/release/nook" "$BIN_DIR/nook"
chmod +x "$BIN_DIR/nook"

APP_DIR="$HOME/.local/share/applications"
mkdir -p "$APP_DIR"
cat > "$APP_DIR/nook.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Nook Enhanced Edition
Comment=Floating Dynamic Island utility hub for Linux
Exec=$BIN_DIR/nook
Terminal=false
Categories=Utility;
EOF

echo "==> Installed to $BIN_DIR/nook"
echo "==> Starting Nook..."
nohup "$BIN_DIR/nook" >/dev/null 2>&1 &
echo "Done!"
