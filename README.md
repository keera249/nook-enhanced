# Nook — Enhanced Dynamic Island Edition (Windows & Linux)

A customized and enhanced fork of **Nook** that turns the top of your screen into an interactive **Dynamic Island / Notch**, featuring a **Smart Game Launcher**, **Auto Mini-Notch in Fullscreen**, **Live Media Pill & Seek Bar**, **Per-App Volume Mixer**, **Power & Performance Switcher**, **Prayer Times & Athkar**, **Dawwar File Converter**, and a full suite of desktop utilities.

---

## Editions & Branch Separation

This repository is organized into two platform editions:

| Edition | Branch | Platform Backend | Bundled Plugins |
|---|---|---|---|
| **Windows Edition** (Windows 10/11) | `main` / `windows` | DirectX 12 (`wgpu`) + Win32 / WinRT APIs | PowerShell + C# (`*.ps1`, `*.cs`) |
| **Linux Edition** (X11 & Wayland) | `linux` (also supported on `main`) | OpenGL (`Glow`) / Vulkan + X11 / MPRIS2 / PulseAudio / `/proc` | Python 3 (`*_linux.py`) |

---

## Core Features

1. **Dynamic Island Media Pill & Full Player**
   - Displays album art, track title, and animated audio waveform when playing media from Spotify, Chrome, Firefox, VLC, etc.
   - Click the media card to open the full player with an interactive **Seek Bar**.
2. **Auto Mini-Notch in Fullscreen / Maximized Windows**
   - Automatically shrinks to a compact mini-notch (`58x14`) when a window is maximized or fullscreen, and expands on hover. Toggle in **Settings → Appearance → `Mini in full window`**.
3. **Smart Game Launcher (`game-launcher`)**
   - **Windows**: Auto-detects Steam, Epic Games, and GOG Galaxy games and supports drag-and-drop for `.exe`, `.lnk`, or game folders.
   - **Linux**: Auto-detects Steam (`~/.local/share/Steam`) and Lutris games, and supports drag-and-drop for executables or `.desktop` files.
4. **Per-App Volume Mixer (`volume-mixer`)**
   - Control master volume and individual app audio streams independently (WASAPI on Windows, `pactl` / PipeWire / PulseAudio on Linux).
5. **Power & Performance Mode Switcher (`power-mode`)**
   - One-click switching between **Power Saver**, **Balanced**, and **Performance** modes.
6. **Prayer Times & Athkar (`prayer-times`)**
   - Live countdown to the next prayer, scheduled Athkar, and an interactive Tasbih counter.
7. **Dawwar File Converter (`dawwar`)**
   - Drag-and-drop file conversion for images, audio, and video.
8. **Island vs. Notch Shape**
   - Switch between a floating **Island** or a top-edge **Notch** in **Settings → Appearance → Shape**.

---

## Windows Edition (Windows 10 / 11)

### Quick Run (Zero-Setup)
1. Download **`Nook-Enhanced-Edition.zip`** or **`nook.exe`** from **[Releases](../../releases)**.
2. Extract (if zipped) and run **`nook.exe`** directly — all bundled plugins are self-provisioned into `%APPDATA%\Nook\plugins` on first launch.

### Building from Source (Windows)
```powershell
git clone https://github.com/keera249/nook-enhanced.git
cd nook-enhanced
cargo build --release
.\target\release\nook.exe
```

---

## Linux Edition (X11 & Wayland)

### Quick Install & Build (Linux)
Switch to the `linux` branch (or use `main`) and run the automated installer script:

```bash
git clone -b linux https://github.com/keera249/nook-enhanced.git
cd nook-enhanced
chmod +x linux/build-and-install.sh
./linux/build-and-install.sh
```

The script installs required system dependencies (`playerctl`, `pulseaudio-utils`, `python3`, `xclip`/`wl-clipboard`, `zenity`, `tesseract-ocr`, `ffmpeg`, X11/OpenGL/Wayland dev headers), builds the release binary, installs it to `~/.local/bin/nook`, creates a desktop entry, and launches Nook.

### Manual Build (Linux)
```bash
cargo build --release
# Optional: include system tray support (requires libgtk-3-dev and libayatana-appindicator3-dev)
cargo build --release --features linux-tray
./target/release/nook
```

All bundled Linux plugins (`*_linux.py`) are automatically provisioned into `~/.config/nook/plugins/` on first launch.

---

## Quick Usage Guide

| Action | How to Use |
|---|---|
| **Open Notch** | Move cursor to top-center of screen, or press `Ctrl + Alt + N` |
| **Pin Notch Open** | Click on the empty glass background |
| **Edit Layout & Tile Sizes** | Open the notch and click the **Edit (Pencil)** icon at the bottom |
| **Quick-Launch a Game** | Left-click any game icon in the Game Launcher tile or page |
| **Switch Island / Notch** | **Settings → Appearance → Shape** (`Island` or `Notch`) |

---

## License

Built on [Nook](https://github.com/Geguchh024/nook) and licensed under the [MIT License](LICENSE).
