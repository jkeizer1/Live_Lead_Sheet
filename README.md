# Live Lead Sheet

A live lead sheet for improvising: it plays a song (chords, rhythm, bass)
to MIDI instruments and shows key, meter, chord names and voicings on a
staff, with the bars ahead in view. See `docs/Concept/Concept Brief.md`.

Revision 1 — 2026-11-01

## The program

```
cargo run --release -p app
```

Opens the window (the spike: 8 bars at 120 quarter notes per minute,
looping).

| Control | Does |
|---|---|
| C | Wheel coloured by function, on and off |
| Click the staff control | Hide or show the pitch staff (EFS-1.5) |
| Esc | Quit |

## Tests

```
cargo test
```

## Checking and measuring programs

Each one's options are in the doc comment at the top of its source file.

| Command | What it does | Output |
|---|---|---|
| `cargo run --release -p render --example layout_preview -- [WIDTH HEIGHT] [hidden] [bar=N]` | Draws one frame on the CPU, without a window or GPU, to check the layout | `scratch/layout_preview.pgm` |
| `cargo run -p alsa_output --example click [CLIENT:PORT]` | Plays a steady click and prints how far ahead of the ALSA queue's clock each one was stamped. Find CLIENT:PORT with `aconnect -l` | Terminal |
| `cargo run --release -p alsa_output --example delivery_spread -- [--bpm N] [--pulses N] [--load N]` | Measures how late the ALSA queue delivers Timing Clock (F8) pulses (D-8) | Terminal |
| `cargo run --release -p alsa_output --example deadline_spread -- [--bpm N] [--pulses N] [--load N] [--spin-us N]` | The same, with the program waiting for each pulse itself (D-8 fallback) | Terminal |

More on the measurements: `crates/alsa_output/examples/explainer.md`.

## Tools

| Command | What it does | Output |
|---|---|---|
| `cargo run --release -p atlas_tool` | Rebuilds the glyph atlas from `assets/fonts`. Run after changing the fonts or the glyph list | `assets/atlas/glyphs.pgm`, `assets/atlas/glyphs.txt` |
| `cargo run --release -p atlas_tool --example check_glyphs` | Checks the distance fields of a few glyphs | `scratch/atlas_check/` |

## Keeping this file current

When a program, example or control is added, changed or removed, update
this file in the same step.
