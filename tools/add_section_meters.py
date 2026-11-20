#!/usr/bin/env python3
"""Writes a meter on every section that has none.

Usage: add_section_meters.py FILE.song [FILE.song ...]

A section no longer takes its meter from the section before it. For each
section with no `meter` and no line for a meter_control track, this adds
`meter n/d` (with its grouping) to the section line's settings, using the
meter of the nearest section above it that has one, else the meter of the
song_composition header. Each file is rewritten in place; keep a copy.
"""

import re
import sys


def code_part(line):
    if line.strip().startswith("#"):
        return ""
    index = line.find("---")
    return line if index < 0 else line[:index]


def process(path):
    with open(path, encoding="utf-8") as handle:
        lines = handle.read().split("\n")
    meter_tracks = set()
    track = None
    for line in lines:
        code = code_part(line)
        m = re.match(r'\s*track\s+"([^"]*)"', code)
        if m:
            track = m.group(1)
        if track and re.match(r"\s*type\s+meter_control\b", code):
            meter_tracks.add(track)
    header_meter = "meter 4/4"
    in_comp = False
    for line in lines:
        code = code_part(line).strip()
        if code == "song_composition":
            in_comp = True
        elif code == "song_composition_close":
            in_comp = False
        elif in_comp:
            m = re.search(r"\bmeter\s+(\S+(?:\s+grouping\s+\S+)?)", code)
            if m and not code.startswith('"'):
                header_meter = "meter " + m.group(1)
    starts = [i for i, l in enumerate(lines) if re.match(r'\s*section\s+"', code_part(l))]
    current = header_meter
    added = 0
    for n, start in enumerate(starts):
        end = starts[n + 1] if n + 1 < len(starts) else len(lines)
        body = [code_part(lines[i]) for i in range(start, end)]
        stop = next((k for k, l in enumerate(body) if re.match(r"\s*song_composition\b", l)), None)
        if stop is not None:
            body = body[:stop]
        text = " ".join(body)
        found = re.search(r"(?<![\w_])meter\s+(\d+/\d+(?:\s+grouping\s+\S+)?)", re.sub(r'"[^"]*"', '""', " ".join(
            l for l in body if not re.match(r"\s*line\s", l))))
        if found:
            current = "meter " + found.group(1)
            continue
        uses_pattern = any(re.match(r'\s*line\s+"%s"' % re.escape(t), l) for t in meter_tracks for l in body)
        if uses_pattern:
            continue
        lines[start] = lines[start].rstrip() + "  " + current
        added += 1
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines))
    print(f"{path}: meter added to {added} sections")


for argument in sys.argv[1:]:
    process(argument)
