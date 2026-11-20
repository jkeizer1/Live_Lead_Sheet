#!/usr/bin/env python3
"""tune_delay.py - measure the PC audio path's latency and write it into a studio file.

    python3 tools/tune_delay.py STUDIO_FILE [--runs N] [--port-skipped NAME]
                                            [--set-ms MS] [--dry-run]

What it does:
  1. Measures the delay from the disting NT's USB input through the PC to the
     L-12, as audio_state_2.py does (buffered samples in ALSA + the measured
     delay inside the devices), N times (default 5), and takes the median.
     Audio must be flowing on that path while it measures.
  2. Rounds the median to whole milliseconds and, in STUDIO_FILE, sets
     `delay_trigger MS ms` on every track block whose port is not the skipped
     port (default "DistingNT"). A track with no channel line is the clock and
     transport track and is left alone. Everything else in the file stays as
     it was.

--set-ms skips the measuring and uses the given whole milliseconds (to set a
value by hand, or to try the editing). --dry-run measures and shows what would
change, and writes nothing.

The measuring code is audio_state_2.py, found at AUDIO_STATE_FILE or at
~/Documents/LinuxAudioNotes/audio_state_2.py.
"""
from __future__ import annotations

import argparse
import importlib.util
import os
import re
import statistics
import sys
from pathlib import Path
from types import ModuleType

DEFAULT_RUNS: int = 5
DEFAULT_SKIPPED_PORT: str = "DistingNT"
DEFAULT_AUDIO_STATE_FILE: Path = Path.home() / "Documents" / "LinuxAudioNotes" / "audio_state_2.py"


class TuneError(Exception):
    """Something went wrong; the message says what to do."""


# ---------------------------------------------------------------- measuring

def load_audio_state() -> ModuleType:
    path: Path = Path(os.environ.get("AUDIO_STATE_FILE", str(DEFAULT_AUDIO_STATE_FILE)))
    if not path.is_file():
        raise TuneError(f"cannot find {path}. Set AUDIO_STATE_FILE to audio_state_2.py.")
    spec = importlib.util.spec_from_file_location("audio_state_2", path)
    if spec is None or spec.loader is None:
        raise TuneError(f"cannot load {path} as a Python file.")
    module: ModuleType = importlib.util.module_from_spec(spec)
    # Its dataclasses look their own module up by name while the file runs, so
    # the module must be registered before it is executed.
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def measure_once_ms(audio: ModuleType) -> float:
    """The total delay, in ms, of the one linked pair whose device delay is known."""
    clock = audio.parse_clock_settings(audio.run_command(["pw-metadata", "-n", "settings"]))
    graph = audio.parse_pw_top(audio.run_command(["pw-top", "-b", "-n", "3"]))
    links = audio.parse_pw_link(audio.run_command(["pw-link", "-l"]))
    alsa = audio.parse_alsa_nodes(audio.run_command(["pw-dump"]))
    running_ids: set[int] = {row.node_id for row in graph}
    running = [node for node in alsa.values() if node.node_id in running_ids]
    live = audio.sample_alsa_live(running)

    pairs: set[tuple[str, str]] = {(link.out_node, link.in_node) for link in links}
    totals: dict[str, float] = {}
    for capture_name, playback_name in sorted(pairs):
        if capture_name not in live or playback_name not in live:
            continue
        source: str = audio.short_name(capture_name)
        destination: str = audio.short_name(playback_name)
        device = audio.DEVICE_DELAYS_SAMPLES.get((source, destination))
        if device is None:
            continue
        sums: list[int] = [c + p for c, p in zip(live[capture_name].delay_readings_samples,
                                                 live[playback_name].delay_readings_samples)]
        samples: float = statistics.fmean(sums) + device[0]
        totals[f"{source} -> {destination}"] = audio.samples_to_ms(samples, clock.rate_hz)
    if not totals:
        raise TuneError("no running, linked pair has a measured device delay. Play audio from the "
                        "NT to the L-12 through the PC and run again.")
    if len(totals) > 1:
        raise TuneError("more than one linked pair has a measured device delay: "
                        + ", ".join(sorted(totals)) + ". Say which one to use.")
    return next(iter(totals.values()))


def measure_ms(runs: int) -> int:
    audio: ModuleType = load_audio_state()
    readings: list[float] = []
    for number in range(1, runs + 1):
        reading: float = measure_once_ms(audio)
        readings.append(reading)
        print(f"  measurement {number} of {runs}: {reading:.1f} ms")
    median: float = statistics.median(readings)
    print(f"  median: {median:.1f} ms")
    return int(median + 0.5)


# ---------------------------------------------------------------- the studio file

TOP_LEVEL_WORD = re.compile(r"^(studio|port|track|destination)\b")
PORT_SETTING = re.compile(r'\bport\s+"([^"]*)"')
DELAY_SETTING = re.compile(r"\bdelay_trigger\s+\d+\s*ms\b")


def code_part(line: str) -> str:
    """The line without its comment, as the studio reader takes it."""
    if line.lstrip().startswith("#"):
        return ""
    cut: int = line.find("---")
    return line if cut < 0 else line[:cut]


def set_delay_triggers(text: str, skipped_port: str, delay_ms: int) -> tuple[str, list[str]]:
    """The studio text with delay_trigger set on each track not on `skipped_port`,
    and a line for each track changed saying what was done."""
    lines: list[str] = text.split("\n")
    starts: list[int] = [i for i, line in enumerate(lines) if TOP_LEVEL_WORD.match(code_part(line))]
    done: list[str] = []
    # From the last block to the first, so inserting a line moves nothing still to do.
    for n in range(len(starts) - 1, -1, -1):
        first: int = starts[n]
        last: int = (starts[n + 1] if n + 1 < len(starts) else len(lines)) - 1
        head = re.match(r'\s*track\s+"([^"]*)"', code_part(lines[first]))
        if head is None:
            continue
        track: str = head.group(1)
        block: str = "\n".join(code_part(line) for line in lines[first:last + 1])
        port = PORT_SETTING.search(block)
        if port is None or re.search(r"\bchannel\b", block) is None:
            continue  # no port, or no channel: the clock and transport track
        if port.group(1) == skipped_port:
            continue
        existing = DELAY_SETTING.search(block)
        new_setting: str = f"delay_trigger {delay_ms} ms"
        if existing is not None:
            if existing.group(0).split()[1] == str(delay_ms):
                done.append(f'{track}: delay_trigger {delay_ms} ms (unchanged)')
                continue
            for i in range(first, last + 1):
                if DELAY_SETTING.search(code_part(lines[i])):
                    lines[i] = DELAY_SETTING.sub(new_setting, lines[i], count=1)
                    break
            done.append(f'{track}: delay_trigger {existing.group(0).split()[1]} ms -> {delay_ms} ms')
        else:
            at: int = last
            while at > first and not code_part(lines[at]).strip():
                at -= 1
            indent: str = re.match(r"\s*", lines[first + 1] if first + 1 <= last else "").group(0) or "  "
            lines.insert(at + 1, f"{indent}{new_setting}")
            done.append(f"{track}: delay_trigger {delay_ms} ms (added)")
    done.reverse()
    return "\n".join(lines), done


# ---------------------------------------------------------------- the command

def main() -> int:
    parser = argparse.ArgumentParser(description="Measure the PC audio latency and write it into a studio file.")
    parser.add_argument("studio_file", type=Path)
    parser.add_argument("--runs", type=int, default=DEFAULT_RUNS)
    parser.add_argument("--port-skipped", default=DEFAULT_SKIPPED_PORT)
    parser.add_argument("--set-ms", type=int, default=None)
    parser.add_argument("--dry-run", action="store_true")
    arguments = parser.parse_args()
    try:
        if arguments.runs < 1:
            raise TuneError("--runs must be 1 or more.")
        if not arguments.studio_file.is_file():
            raise TuneError(f"cannot read the studio file {arguments.studio_file}.")
        print(f"\n{arguments.studio_file.name}")
        if arguments.set_ms is not None:
            delay_ms: int = arguments.set_ms
        else:
            print("Measuring the PC audio path (audio must be flowing):")
            delay_ms = measure_ms(arguments.runs)
        original: str = arguments.studio_file.read_text(encoding="utf-8")
        changed, report = set_delay_triggers(original, arguments.port_skipped, delay_ms)
        if not report:
            raise TuneError(f'no track outside the port "{arguments.port_skipped}" has a channel, so '
                            "nothing was changed.")
        print(f"\nDelay: {delay_ms} ms. Tracks not on the port \"{arguments.port_skipped}\":")
        for line in report:
            print(f"  {line}")
        if arguments.dry_run:
            print("\nDry run: the file was not changed.")
        elif changed == original:
            print("\nThe file already had these values.")
        else:
            arguments.studio_file.write_text(changed, encoding="utf-8")
            print(f"\n{arguments.studio_file.name} written.")
        return 0
    except TuneError as error:
        print(f"\nError: {error}", file=sys.stderr)
        return 1
    except Exception as error:  # noqa: BLE001 - a measuring tool: say what failed, with the step's text
        print(f"\nError: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
