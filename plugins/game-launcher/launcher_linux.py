#!/usr/bin/env python3
"""Smart Game Launcher for Nook on Linux (Steam, Heroic, Lutris, .desktop & custom)."""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

tile_size = "wide"
data_dir = Path(os.environ.get("NOOK_DATA_DIR", Path(__file__).parent / ".data"))
data_dir.mkdir(parents=True, exist_ok=True)
custom_file = data_dir / "custom_games.json"
games = []


def send(msg):
    sys.stdout.write(json.dumps(msg, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def load_custom():
    if custom_file.is_file():
        try:
            return json.loads(custom_file.read_text(encoding="utf-8"))
        except Exception:
            pass
    return []


def save_custom(items):
    custom_file.write_text(json.dumps(items, indent=2), encoding="utf-8")


def scan_steam():
    found = []
    home = Path.home()
    roots = [
        home / ".local/share/Steam/steamapps",
        home / ".steam/steam/steamapps",
        home / ".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps",
    ]
    seen = set()
    for root in roots:
        if not root.is_dir():
            continue
        for acf in root.glob("appmanifest_*.acf"):
            try:
                txt = acf.read_text(encoding="utf-8", errors="ignore")
                aid = re.search(r'"appid"\s+"(\d+)"', txt)
                name = re.search(r'"name"\s+"([^"]+)"', txt)
                if aid and name:
                    gid = aid.group(1)
                    if gid in seen or gid == "228980":
                        continue
                    seen.add(gid)
                    found.append({
                        "id": f"steam:{gid}",
                        "name": name.group(1),
                        "source": "Steam",
                        "cmd": ["xdg-open", f"steam://rungameid/{gid}"],
                    })
            except Exception:
                continue
    return found


def scan_lutris():
    found = []
    lutris_dir = Path.home() / ".config/lutris/games"
    if not lutris_dir.is_dir():
        return found
    for yml in lutris_dir.glob("*.yml"):
        try:
            txt = yml.read_text(encoding="utf-8", errors="ignore")
            nm = re.search(r"^name:\s*(.+)$", txt, re.MULTILINE)
            slug = re.search(r"^slug:\s*(.+)$", txt, re.MULTILINE)
            if nm and slug:
                s = slug.group(1).strip()
                found.append({
                    "id": f"lutris:{s}",
                    "name": nm.group(1).strip(),
                    "source": "Lutris",
                    "cmd": ["lutris", f"lutris:rungame/{s}"],
                })
        except Exception:
            continue
    return found


def refresh_games():
    global games
    items = []
    items.extend(load_custom())
    items.extend(scan_steam())
    items.extend(scan_lutris())
    games = items


def launch_game(gid):
    for g in games:
        if g["id"] == gid:
            try:
                subprocess.Popen(g["cmd"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            except Exception:
                pass
            return


def add_dropped(paths):
    custom = load_custom()
    for p in paths:
        path = Path(p)
        if not path.exists():
            continue
        name = path.stem
        cmd = ["xdg-open", str(path)] if path.suffix == ".desktop" else [str(path)]
        custom.insert(0, {
            "id": f"custom:{len(custom)}:{name}",
            "name": name,
            "source": "Custom",
            "cmd": cmd,
        })
    save_custom(custom)
    refresh_games()


def draw_tile():
    count = len(games)
    if tile_size == "small":
        send({
            "type": "tile",
            "view": {
                "type": "column",
                "align": "center",
                "gap": 2,
                "children": [
                    {"type": "icon", "icon": "game-controller", "size": 22, "color": "accent"},
                    {"type": "text", "text": f"{count}", "size": "small", "color": "dim"},
                ],
            },
        })
        return

    btns = []
    for g in games[:4]:
        btns.append({
            "type": "button",
            "id": f"launch:{g['id']}",
            "label": g["name"][:12],
            "icon": "play",
            "style": "ghost",
        })
    if not btns:
        btns.append({"type": "text", "text": "Drop game or install Steam/Lutris", "size": "small", "color": "dim"})

    send({
        "type": "tile",
        "view": {
            "type": "column",
            "gap": 6,
            "children": [
                {"type": "row", "align": "center", "gap": 6, "children": [
                    {"type": "icon", "icon": "game-controller", "size": 18, "color": "accent"},
                    {"type": "text", "text": f"Games ({count})", "size": "title"},
                ]},
                {"type": "row", "gap": 6, "children": btns},
            ],
        },
    })


def draw_page():
    children = [
        {"type": "row", "align": "center", "gap": 8, "children": [
            {"type": "icon", "icon": "game-controller", "size": 20, "color": "accent"},
            {"type": "text", "text": f"Game Launcher ({len(games)})", "size": "title"},
            {"type": "spacer"},
            {"type": "button", "id": "refresh", "icon": "arrow-clockwise", "tooltip": "Rescan"},
        ]},
        {"type": "divider"},
    ]
    if not games:
        children.append({"type": "text", "text": "No Steam/Lutris games detected yet. Drop an executable or .desktop file here.", "size": "small", "color": "dim"})
    else:
        for g in games[:24]:
            children.append({
                "type": "row",
                "align": "center",
                "gap": 8,
                "children": [
                    {"type": "icon", "icon": "game-controller", "size": 18, "color": "accent"},
                    {"type": "text", "text": g["name"], "size": "body"},
                    {"type": "text", "text": g["source"], "size": "small", "color": "dim"},
                    {"type": "spacer"},
                    {"type": "button", "id": f"launch:{g['id']}", "label": "Play", "icon": "play", "style": "accent"},
                ],
            })
    send({"type": "page", "view": {"type": "column", "gap": 8, "children": children}})


def main():
    global tile_size
    refresh_games()
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except Exception:
            continue
        t = msg.get("type")
        if t == "init":
            tile_size = msg.get("size", "wide")
            refresh_games()
            draw_tile()
        elif t == "resize":
            tile_size = msg.get("size", tile_size)
            draw_tile()
        elif t == "open":
            refresh_games()
            draw_page()
        elif t == "drop":
            add_dropped(msg.get("paths", []))
            draw_tile()
            draw_page()
        elif t == "action":
            aid = msg.get("id", "")
            if aid.startswith("launch:"):
                launch_game(aid.split(":", 1)[1])
            elif aid == "refresh":
                refresh_games()
                draw_tile()
                draw_page()


if __name__ == "__main__":
    main()
