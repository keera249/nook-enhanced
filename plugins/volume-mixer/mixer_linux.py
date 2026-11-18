#!/usr/bin/env python3
"""Per-App Volume Mixer for Nook on Linux (PulseAudio / PipeWire via pactl)."""

import json
import re
import subprocess
import sys

tile_size = "small"
master_vol = 100
master_muted = False
apps = []


def send(msg):
    sys.stdout.write(json.dumps(msg, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def run_pactl(args):
    try:
        return subprocess.check_output(["pactl", *args], text=True, stderr=subprocess.DEVNULL)
    except Exception:
        return ""


def refresh_audio():
    global master_vol, master_muted, apps
    vol_out = run_pactl(["get-sink-volume", "@DEFAULT_SINK@"])
    m = re.search(r"(\d+)%", vol_out)
    if m:
        master_vol = min(150, int(m.group(1)))
    mute_out = run_pactl(["get-sink-mute", "@DEFAULT_SINK@"])
    master_muted = "yes" in mute_out.lower()

    raw = run_pactl(["list", "sink-inputs"])
    parsed = []
    cur = None
    for line in raw.splitlines():
        hdr = re.match(r"^Sink Input #(\d+)", line)
        if hdr:
            if cur:
                parsed.append(cur)
            cur = {"id": hdr.group(1), "name": f"Stream #{hdr.group(1)}", "vol": 100, "muted": False}
            continue
        if not cur:
            continue
        if "Mute:" in line:
            cur["muted"] = "yes" in line.lower()
        elif "Volume:" in line:
            vm = re.search(r"(\d+)%", line)
            if vm:
                cur["vol"] = min(150, int(vm.group(1)))
        elif "application.name =" in line:
            nm = re.search(r'application\.name\s*=\s*"([^"]+)"', line)
            if nm:
                cur["name"] = nm.group(1)
    if cur:
        parsed.append(cur)
    apps = parsed


def draw_tile():
    m_icon = "speaker-slash" if (master_muted or master_vol == 0) else ("speaker-low" if master_vol < 50 else "speaker-high")
    m_color = "red" if master_muted else "accent"
    vol_txt = "Muted" if master_muted else f"{master_vol}%"
    if tile_size == "small":
        send({
            "type": "tile",
            "view": {
                "type": "ring",
                "value": round(min(master_vol, 100) / 100.0, 2),
                "label": str(master_vol),
                "color": m_color,
                "tooltip": f"Volume Mixer ({len(apps)} apps)",
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
                    {"type": "icon", "icon": "faders", "size": 22, "color": m_color},
                    {"type": "column", "gap": 0, "children": [
                        {"type": "text", "text": f"Mixer ({len(apps)})", "size": "small", "color": "dim"},
                        {"type": "text", "text": vol_txt, "size": "title"},
                    ]},
                    {"type": "spacer"},
                    {"type": "button", "id": "mute:master", "icon": m_icon, "tooltip": "Mute / Unmute Master"},
                ],
            },
        })


def draw_page():
    m_icon = "speaker-slash" if master_muted else "speaker-high"
    children = [
        {"type": "row", "align": "center", "gap": 8, "children": [
            {"type": "icon", "icon": "faders", "size": 20, "color": "accent"},
            {"type": "text", "text": "App Volume Mixer", "size": "title"},
            {"type": "spacer"},
            {"type": "button", "id": "refresh", "icon": "arrow-clockwise", "tooltip": "Refresh"},
        ]},
        {"type": "divider"},
        {"type": "row", "align": "center", "gap": 8, "children": [
            {"type": "icon", "icon": m_icon, "size": 20, "color": "red" if master_muted else "accent"},
            {"type": "text", "text": "Master Volume", "size": "body"},
            {"type": "spacer"},
            {"type": "button", "id": "vol:master:-10", "icon": "minus"},
            {"type": "text", "text": f"{master_vol}%", "size": "body"},
            {"type": "button", "id": "vol:master:+10", "icon": "plus"},
            {"type": "button", "id": "mute:master", "label": "Unmute" if master_muted else "Mute", "style": "accent" if master_muted else "ghost"},
        ]},
        {"type": "divider"},
    ]
    if not apps:
        children.append({"type": "text", "text": "No active audio streams.", "size": "small", "color": "dim"})
    else:
        for a in apps:
            sid = a["id"]
            muted = a["muted"]
            children.append({
                "type": "row",
                "align": "center",
                "gap": 8,
                "children": [
                    {"type": "icon", "icon": "speaker-slash" if muted else "speaker-high", "size": 18, "color": "red" if muted else "accent"},
                    {"type": "text", "text": a["name"], "size": "body"},
                    {"type": "spacer"},
                    {"type": "button", "id": f"vol:{sid}:-10", "icon": "minus"},
                    {"type": "text", "text": f"{a['vol']}%", "size": "small"},
                    {"type": "button", "id": f"vol:{sid}:+10", "icon": "plus"},
                    {"type": "button", "id": f"mute:{sid}", "label": "Unmute" if muted else "Mute", "style": "accent" if muted else "ghost"},
                ],
            })
    send({"type": "page", "view": {"type": "column", "gap": 8, "children": children}})


def handle_action(aid):
    if aid == "mute:master":
        run_pactl(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
    elif aid.startswith("vol:master:"):
        delta = aid.split(":")[2]
        run_pactl(["set-sink-volume", "@DEFAULT_SINK@", f"{delta}%"])
    elif aid.startswith("mute:"):
        sid = aid.split(":")[1]
        run_pactl(["set-sink-input-mute", sid, "toggle"])
    elif aid.startswith("vol:"):
        _, sid, delta = aid.split(":")
        run_pactl(["set-sink-input-volume", sid, f"{delta}%"])


def main():
    global tile_size
    refresh_audio()
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
            refresh_audio()
            draw_tile()
        elif t == "resize":
            tile_size = msg.get("size", tile_size)
            draw_tile()
        elif t == "open":
            refresh_audio()
            draw_page()
        elif t == "action":
            handle_action(msg.get("id", ""))
            refresh_audio()
            draw_tile()
            draw_page()


if __name__ == "__main__":
    main()
