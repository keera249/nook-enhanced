#!/usr/bin/env python3
"""Islamic Prayer Times & Athkar plugin for Nook on Linux."""

import datetime
import json
import math
import sys
import urllib.request
from pathlib import Path

tile_size = "small"
athkar_idx = 0
tasbih_count = 0
athkar_list = []

athkar_path = Path(__file__).parent / "athkar.json"
if athkar_path.is_file():
    try:
        athkar_list = json.loads(athkar_path.read_text(encoding="utf-8"))
    except Exception:
        athkar_list = []

if not athkar_list:
    athkar_list = [
        {"text": "سبحان الله وبحمده", "note": "100 مرة"},
        {"text": "أستغفر الله العظيم وأتوب إليه", "note": "الاستغفار"},
        {"text": "لا حول ولا قوة إلا بالله", "note": "كنز من كنوز الجنة"},
    ]

# Default prayer times (fetched via Aladhan API if online, fallback to approximate schedule)
timings = {"Fajr": "04:45", "Dhuhr": "12:15", "Asr": "15:35", "Maghrib": "18:05", "Isha": "19:25"}
city_label = "Auto"


def send(msg):
    sys.stdout.write(json.dumps(msg, ensure_ascii=False, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def fetch_timings():
    global timings, city_label
    try:
        with urllib.request.urlopen("https://ipapi.co/json/", timeout=3) as r:
            loc = json.loads(r.read().decode("utf-8"))
        lat = loc.get("latitude")
        lon = loc.get("longitude")
        city_label = loc.get("city", "Local")
        if lat and lon:
            url = f"https://api.aladhan.com/v1/timings?latitude={lat}&longitude={lon}&method=4"
            with urllib.request.urlopen(url, timeout=4) as r2:
                data = json.loads(r2.read().decode("utf-8"))["data"]["timings"]
            timings = {k: data[k][:5] for k in ("Fajr", "Dhuhr", "Asr", "Maghrib", "Isha") if k in data}
    except Exception:
        pass


def next_prayer():
    now = datetime.datetime.now()
    now_mins = now.hour * 60 + now.minute
    for name in ("Fajr", "Dhuhr", "Asr", "Maghrib", "Isha"):
        hh, mm = map(int, timings[name].split(":"))
        pm = hh * 60 + mm
        if pm > now_mins:
            diff = pm - now_mins
            return name, timings[name], f"{diff // 60}h {diff % 60}m", max(0.05, 1.0 - diff / 360.0)
    hh, mm = map(int, timings["Fajr"].split(":"))
    diff = (24 * 60 - now_mins) + (hh * 60 + mm)
    return "Fajr", timings["Fajr"], f"{diff // 60}h {diff % 60}m", 0.15


def draw_tile():
    name, time_str, rem, prog = next_prayer()
    if tile_size == "small":
        send({
            "type": "tile",
            "view": {
                "type": "ring",
                "value": round(prog, 2),
                "label": name[:3],
                "color": "accent",
                "tooltip": f"{name} {time_str} ({rem})",
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
                    {"type": "icon", "icon": "moon-stars", "size": 22, "color": "accent"},
                    {"type": "column", "gap": 0, "children": [
                        {"type": "text", "text": f"Next: {name} ({time_str})", "size": "small", "color": "dim"},
                        {"type": "text", "text": f"in {rem}", "size": "title"},
                    ]},
                ],
            },
        })


def draw_page():
    name, time_str, rem, _ = next_prayer()
    thikr = athkar_list[athkar_idx % len(athkar_list)]
    thikr_txt = thikr.get("text", str(thikr)) if isinstance(thikr, dict) else str(thikr)
    rows = []
    for k in ("Fajr", "Dhuhr", "Asr", "Maghrib", "Isha"):
        is_next = k == name
        rows.append({
            "type": "row",
            "align": "center",
            "children": [
                {"type": "text", "text": k, "size": "body", "color": "accent" if is_next else "default"},
                {"type": "spacer"},
                {"type": "text", "text": timings[k], "size": "body", "color": "accent" if is_next else "dim"},
            ],
        })
    send({
        "type": "page",
        "view": {
            "type": "column",
            "gap": 10,
            "children": [
                {"type": "row", "align": "center", "gap": 8, "children": [
                    {"type": "icon", "icon": "moon-stars", "size": 20, "color": "accent"},
                    {"type": "text", "text": f"Prayer Times ({city_label})", "size": "title"},
                    {"type": "spacer"},
                    {"type": "text", "text": f"{name} in {rem}", "size": "small", "color": "accent"},
                ]},
                {"type": "divider"},
                *rows,
                {"type": "divider"},
                {"type": "text", "text": thikr_txt, "size": "body"},
                {"type": "row", "align": "center", "gap": 8, "children": [
                    {"type": "button", "id": "tasbih", "label": f"Tasbih ({tasbih_count})", "style": "accent"},
                    {"type": "button", "id": "next_thikr", "label": "Next Thikr", "style": "ghost"},
                    {"type": "button", "id": "reset_tasbih", "label": "Reset", "style": "ghost"},
                ]},
            ],
        },
    })


def main():
    global tile_size, athkar_idx, tasbih_count
    fetch_timings()
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
            draw_tile()
        elif t == "resize":
            tile_size = msg.get("size", tile_size)
            draw_tile()
        elif t == "open":
            draw_page()
        elif t == "action":
            aid = msg.get("id", "")
            if aid == "tasbih":
                tasbih_count += 1
            elif aid == "next_thikr":
                athkar_idx += 1
            elif aid == "reset_tasbih":
                tasbih_count = 0
            draw_page()


if __name__ == "__main__":
    main()
