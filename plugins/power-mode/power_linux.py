#!/usr/bin/env python3
"""Power & Performance Switcher for Nook on Linux (powerprofilesctl / ACPI)."""

import json
import os
import subprocess
import sys

tile_size = "small"
current_mode = "balanced"
status_msg = None


def send(msg):
    sys.stdout.write(json.dumps(msg, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def get_mode():
    try:
        out = subprocess.check_output(["powerprofilesctl", "get"], text=True, stderr=subprocess.DEVNULL).strip()
        if out in ("power-saver", "balanced", "performance"):
            return out
    except Exception:
        pass
    acpi = "/sys/firmware/acpi/platform_profile"
    if os.path.isfile(acpi):
        try:
            with open(acpi, "r", encoding="utf-8") as f:
                val = f.read().strip()
                if "low-power" in val or "quiet" in val:
                    return "power-saver"
                if "performance" in val:
                    return "performance"
                return "balanced"
        except Exception:
            pass
    return "balanced"


def set_mode(mode):
    global current_mode, status_msg
    try:
        subprocess.check_call(["powerprofilesctl", "set", mode], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        current_mode = mode
        status_msg = {"text": f"Switched Power Mode to {mode}", "color": "green"}
        return
    except Exception:
        pass
    acpi = "/sys/firmware/acpi/platform_profile"
    if os.path.isfile(acpi):
        mapped = {"power-saver": "low-power", "balanced": "balanced", "performance": "performance"}.get(mode, "balanced")
        try:
            with open(acpi, "w", encoding="utf-8") as f:
                f.write(mapped)
            current_mode = mode
            status_msg = {"text": f"Switched ACPI profile to {mapped}", "color": "green"}
            return
        except Exception as e:
            status_msg = {"text": f"Failed: {e}", "color": "orange"}
            return
    status_msg = {"text": "powerprofilesctl not installed (install power-profiles-daemon)", "color": "orange"}


def draw_tile():
    color = {"power-saver": "green", "balanced": "accent", "performance": "red"}.get(current_mode, "accent")
    label = {"power-saver": "Eco", "balanced": "Balanced", "performance": "Perf"}.get(current_mode, "Balanced")
    if tile_size == "small":
        send({
            "type": "tile",
            "view": {
                "type": "column",
                "align": "center",
                "gap": 2,
                "children": [
                    {"type": "icon", "icon": "lightning", "size": 22, "color": color},
                    {"type": "text", "text": label, "size": "small", "color": color},
                ],
            },
        })
    else:
        send({
            "type": "tile",
            "view": {
                "type": "row",
                "align": "center",
                "gap": 8,
                "children": [
                    {"type": "icon", "icon": "lightning", "size": 22, "color": color},
                    {"type": "column", "gap": 0, "children": [
                        {"type": "text", "text": "Power Mode", "size": "small", "color": "dim"},
                        {"type": "text", "text": label, "size": "title", "color": color},
                    ]},
                    {"type": "spacer"},
                    {"type": "button", "id": "mode:power-saver", "label": "Eco", "style": "accent" if current_mode == "power-saver" else "ghost"},
                    {"type": "button", "id": "mode:balanced", "label": "Bal", "style": "accent" if current_mode == "balanced" else "ghost"},
                    {"type": "button", "id": "mode:performance", "label": "Perf", "style": "accent" if current_mode == "performance" else "ghost"},
                ],
            },
        })


def draw_page():
    children = [
        {"type": "row", "align": "center", "gap": 8, "children": [
            {"type": "icon", "icon": "lightning", "size": 20, "color": "accent"},
            {"type": "text", "text": "Linux Power & Performance", "size": "title"},
            {"type": "spacer"},
            {"type": "button", "id": "refresh", "icon": "arrow-clockwise", "tooltip": "Refresh"},
        ]},
        {"type": "divider"},
        {"type": "text", "text": "Select Active Power Profile", "size": "small", "color": "dim"},
        {"type": "row", "gap": 8, "children": [
            {"type": "button", "id": "mode:power-saver", "label": "Power Saver", "icon": "leaf", "style": "accent" if current_mode == "power-saver" else "ghost"},
            {"type": "button", "id": "mode:balanced", "label": "Balanced", "icon": "scales", "style": "accent" if current_mode == "balanced" else "ghost"},
            {"type": "button", "id": "mode:performance", "label": "Performance", "icon": "flame", "style": "accent" if current_mode == "performance" else "ghost"},
        ]},
    ]
    if status_msg:
        children.append({"type": "text", "text": status_msg["text"], "size": "small", "color": status_msg["color"]})
    send({"type": "page", "view": {"type": "column", "gap": 10, "children": children}})


def main():
    global tile_size, current_mode
    current_mode = get_mode()
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
            tile_size = msg.get("size", "small")
            current_mode = get_mode()
            draw_tile()
        elif t == "resize":
            tile_size = msg.get("size", tile_size)
            draw_tile()
        elif t == "open":
            current_mode = get_mode()
            draw_page()
        elif t == "action":
            aid = msg.get("id", "")
            if aid.startswith("mode:"):
                set_mode(aid.split(":", 1)[1])
            elif aid == "refresh":
                current_mode = get_mode()
            draw_tile()
            draw_page()


if __name__ == "__main__":
    main()
