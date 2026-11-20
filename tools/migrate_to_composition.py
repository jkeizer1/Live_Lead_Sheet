#!/usr/bin/env python3
"""Moves a .song file from `repeat n` on its sections to a song_composition block.

Usage: migrate_to_composition.py [--at-end] [--default-header] FILE.song [FILE.song ...]

For each file:
  * every `repeat n` is taken off the section lines (a section is a definition);
  * a section written with no lines that names an earlier section and writes
    nothing but `repeat n` (it played that section again) is removed, and its
    place in the song becomes an entry of the composition;
  * a song_composition block is written listing the sections in the order they
    played, with `repeat n` where n is not 1 (and `repeat 0` where the section
    was left out). Its starting key, meter and tempo are those of the first
    sections that write them (key C major, meter 4/4, tempo 100 beats/min
    where no section does).

With --default-header the starting values are always key C major, meter 4/4,
tempo 100 beats/min (for test files whose sections may be deliberately wrong).

By default the block goes just before the first section. With --at-end it goes
at the end of the file and removed lines are replaced by comments, so the line
numbers of everything above stay the same.

Files that already have a song_composition are left alone. Each file is
rewritten in place; keep a copy first if you want one.
"""

import re
import sys

SETTING_WORDS = ("key", "meter", "tempo", "count-in", "repeat", "grouping")


def code_part(line):
    """The line without its comment, as the loader reads it."""
    stripped = line.strip()
    if stripped.startswith("#"):
        return ""
    index = line.find("---")
    return line if index < 0 else line[:index]


def is_section_line(line):
    return re.match(r"\s*section\s+\"", line) is not None


def is_setting_line(line):
    words = code_part(line).split()
    return bool(words) and words[0] in SETTING_WORDS


def migrate(path, at_end, default_header):
    with open(path, encoding="utf-8") as handle:
        lines = handle.read().split("\n")
    if any(re.match(r"\s*song_composition\b", code_part(line)) for line in lines):
        print(f"{path}: already has a song_composition, left alone")
        return

    # Sections: (index of section line, name)
    starts = [i for i, line in enumerate(lines) if is_section_line(line)]
    if not starts:
        print(f"{path}: no sections, left alone")
        return
    blocks = []
    for n, start in enumerate(starts):
        end = starts[n + 1] if n + 1 < len(starts) else len(lines)
        name = re.match(r"\s*section\s+\"([^\"]*)\"", lines[start]).group(1)
        has_lines = any(
            re.match(r"\s*line\s", code_part(lines[i])) for i in range(start, end)
        )
        settings_text = " ".join(
            code_part(lines[i])
            for i in range(start, end)
            if i == start or is_setting_line(lines[i])
        )
        settings_text = re.sub(r'"[^"]*"', '""', settings_text)
        repeat_match = re.search(r"\brepeat\s+(\d+)", settings_text)
        repeat = int(repeat_match.group(1)) if repeat_match else 1
        blocks.append((start, end, name, has_lines, settings_text, repeat))

    # A section with no lines plays an earlier one again only if it names one
    # and writes nothing but repeat; otherwise it stays as written.
    seen_with_lines = set()
    checked = []
    for start, end, name, has_lines, text, repeat in blocks:
        plain = re.sub(r'"[^"]*"', '""', text)
        only_repeat = re.sub(r"\bsection\b|\brepeat\s+\d+|\"\"", "", plain).strip() == ""
        is_again = (not has_lines) and name in seen_with_lines and only_repeat
        if has_lines:
            seen_with_lines.add(name)
        checked.append((start, end, name, has_lines, text, repeat, is_again))
    blocks = checked

    # The starting key, meter and tempo: the first section to write each.
    key = meter = grouping = tempo = None
    for _, _, _, has_lines, text, _, _ in blocks:
        if not has_lines or default_header:
            continue
        found = re.search(r"\bkey\s+(\S+)\s+(\S+)", text)
        if key is None and found:
            key = f"key {found.group(1)} {found.group(2)}"
        found = re.search(r"\bmeter\s+(\S+)(\s+grouping\s+\S+)?", text)
        if meter is None and found:
            meter = f"meter {found.group(1)}"
            grouping = found.group(2).strip() if found.group(2) else None
        found = re.search(r"\btempo\s+([\d.]+)\s+(beats/\w+)", text)
        if tempo is None and found:
            tempo = f"tempo {found.group(1)} {found.group(2)}"
    key = key or "key C major"
    meter = meter or "meter 4/4"
    tempo = tempo or "tempo 100 beats/min"
    start_line = f"  {key}  {meter}{' ' + grouping if grouping else ''}  {tempo}"

    entries = []
    removed = set()
    for start, end, name, has_lines, _, repeat, is_again in blocks:
        entries.append(f'  "{name}"' + (f"  repeat {repeat}" if repeat != 1 else ""))
        # Take `repeat n` off the section line and its settings lines.
        for i in range(start, end):
            if i == start or is_setting_line(lines[i]):
                lines[i] = re.sub(r"\s*\brepeat\s+\d+", "", lines[i], count=1) if "repeat" in code_part(lines[i]) else lines[i]
        if is_again:
            for i in range(start, end):
                if i == start or is_setting_line(lines[i]):
                    removed.add(i)

    block = ["song_composition", start_line] + entries + ["song_composition_close", ""]

    out = []
    if at_end:
        for i, line in enumerate(lines):
            if i in removed:
                out.append("# (played again by the song_composition below)")
            else:
                out.append(line)
        while out and out[-1] == "":
            out.pop()
        out.append("")
        out.extend(block)
    else:
        first = starts[0]
        for i, line in enumerate(lines):
            if i == first:
                out.extend(block)
            if i in removed:
                continue
            out.append(line)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("\n".join(out))
    print(f"{path}: {len(blocks)} sections, {len(entries)} entries")


def main():
    arguments = sys.argv[1:]
    at_end = "--at-end" in arguments
    default_header = "--default-header" in arguments
    for path in [a for a in arguments if not a.startswith("--")]:
        migrate(path, at_end, default_header)


if __name__ == "__main__":
    main()
