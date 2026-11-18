#!/usr/bin/env python3
"""Dawwar file converter plugin for Nook on Linux (ffmpeg / ImageMagick)."""

import json
import subprocess
import sys
from pathlib import Path

tile_size = "wide"
queued_files = []
status_msg = "Drop any image, audio, or video file to convert."


def send(msg):
    sys.stdout.write(json.dumps(msg, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def convert_file(src_path, target_ext):
    global status_msg
    src = Path(src_path)
    dst = src.with_suffix(f".{target_ext}")
    try:
        subprocess.check_call(
            ["ffmpeg", "-y", "-i", str(src), str(dst)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        status_msg = f"Converted -> {dst.name}"
    except FileNotFoundError:
        status_msg = "Install ffmpeg (sudo apt install ffmpeg) to convert files."
    except Exception as e:
        status_msg = f"Conversion failed: {e}"


def draw_tile():
    if tile_size == "small":
        send({
            "type": "tile",
            "view": {
                "type": "column",
                "align": "center",
                "gap": 2,
                "children": [
                    {"type": "icon", "icon": "aperture", "size": 22, "color": "accent"},
                    {"type": "text", "text": "Dawwar", "size": "small", "color": "dim"},
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
                    {"type": "icon", "icon": "aperture", "size": 22, "color": "accent"},
                    {"type": "column", "gap": 0, "children": [
                        {"type": "text", "text": "Dawwar Converter", "size": "title"},
                        {"type": "text", "text": status_msg[:38], "size": "small", "color": "dim"},
                    ]},
                ],
            },
        })


def draw_page():
    children = [
        {"type": "row", "align": "center", "gap": 8, "children": [
            {"type": "icon", "icon": "aperture", "size": 20, "color": "accent"},
            {"type": "text", "text": "Dawwar File Converter", "size": "title"},
        ]},
        {"type": "divider"},
        {"type": "text", "text": status_msg, "size": "small", "color": "accent"},
    ]
    if queued_files:
        f = Path(queued_files[0])
        children.append({"type": "text", "text": f"Selected: {f.name}", "size": "body"})
        children.append({
            "type": "row",
            "gap": 6,
            "children": [
                {"type": "button", "id": "conv:png", "label": "PNG", "style": "ghost"},
                {"type": "button", "id": "conv:jpg", "label": "JPG", "style": "ghost"},
                {"type": "button", "id": "conv:webp", "label": "WEBP", "style": "ghost"},
                {"type": "button", "id": "conv:mp3", "label": "MP3", "style": "ghost"},
                {"type": "button", "id": "conv:mp4", "label": "MP4", "style": "ghost"},
            ],
        })
    send({"type": "page", "view": {"type": "column", "gap": 10, "children": children}})


def main():
    global tile_size, queued_files, status_msg
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
            draw_tile()
        elif t == "resize":
            tile_size = msg.get("size", tile_size)
            draw_tile()
        elif t == "open":
            draw_page()
        elif t == "drop":
            queued_files = msg.get("paths", [])
            if queued_files:
                status_msg = f"Ready: {Path(queued_files[0]).name}"
            draw_tile()
            draw_page()
        elif t == "action":
            aid = msg.get("id", "")
            if aid.startswith("conv:") and queued_files:
                convert_file(queued_files[0], aid.split(":", 1)[1])
                draw_tile()
                draw_page()


if __name__ == "__main__":
    main()
