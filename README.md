# Live Lead Sheet

A live lead sheet for improvising: it plays a song (chords, rhythm, bass)
to MIDI instruments and shows key, meter, chord names and voicings on a
staff, with the bars ahead in view. See `docs/Concept/Concept Brief.md`.

Revision 6 — 2026-11-08

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
A name ending in `_test` is a test, not part of the product; each test
has its own folder under `examples/`, with `main.rs` and an
`explainer.md` (what it proves, findings, how to run it, date).

| Command | What it does | Output |
|---|---|---|
| `cargo run --release -p render --example layout_preview -- [WIDTH HEIGHT] [hidden] [bar=N]` | Draws one frame on the CPU, without a window or GPU, to check the layout | `scratch/layout_preview.pgm` |
| `cargo run --release -p alsa_output --example control_track_test -- [options]` | Test: plays a count-in on the beat pulse, then a reset trigger and an SQ-1 Start on the music's first bar, with Timing Clock on its own port (D-10); optional SQ-1 restarts. Asks you to connect its ports, then waits for Enter. See its `explainer.md` | Terminal |
| `cargo run --release -p song_text --example rhythm_notation_test -- [LIBRARY_FILE] [--play "Name" [--bpm N] [--bars N] [--channel C] [--note N] [--transport]]` | Test: loads `library/Rhythms.library` (or the file given), prints every rhythm in beats and note values, and checks that six kinds of mistake are reported as EFS-6.3 asks. With `--play`, plays that rhythm on the beat-pulse port (Timing Clock on its own port) and asks you to connect the ports first; `--transport` adds Start and Stop, which the FH-2's clock output needs. See its `explainer.md` | Terminal |
| `cargo run -p song_text --example song_notation_test -- [SONG_FILE [STUDIO_FILE]]` | Test: loads `songs/Demo.song` and `songs/Demo.studio` (or the files given), prints the song track by track and bar by bar in beats and note values, checks that the studio gives every track and cue destination the song needs, and checks that nine kinds of mistake are reported as EFS-6.3 asks. See its `explainer.md` | Terminal |
| `cargo run --release -p song_text --example song_play_test -- [SONG_FILE [STUDIO_FILE]]` | Test: plays `songs/Demo.song` through `songs/Demo.studio` (or the files given): count-in, beat, chords, bass, the Start cue, with Timing Clock, Start and Stop on the studio's clock port. A note that ends where the next note on its track starts is sent 5 ms short (more with the studio's `retrigger_gap`). Opens one output port for each port the studio lists, names them and asks you to connect them first. See its `explainer.md` | Terminal |
| `cargo run -p lilypond_export --example lilypond_export_test -- SONG_FILE [OUTPUT_FILE]` | Test: writes the song as a LilyPond file, to read it as sheet music; then `lilypond -o ~/tmp ~/tmp/NAME.ly` engraves it (LilyPond is installed separately: `sudo apt install lilypond`). Draws plain and dotted values and triplets; anything else is reported with its section, track and bar. See its `explainer.md` | `~/tmp/NAME.ly` (or OUTPUT_FILE), then `~/tmp/NAME.pdf` |
| `cargo run --release -p alsa_output --example retrigger_gap_test -- [options]` | Test: sends one repeated note on the port `Bass`, in groups of eight eighth notes with a gap before each next note on of 0, 960, 1100, 1250, 1400 and 1600 µs (or `--gaps-us`); `--on-before-off` reverses the order of a touching note off and note on. Finds what a device needs to retrigger its gate. See its `explainer.md` | Terminal |
| `cargo run -p alsa_output --example click [CLIENT:PORT]` | Plays a steady click and prints how far ahead of the ALSA queue's clock each one was stamped. Find CLIENT:PORT with `aconnect -l` | Terminal |
| `cargo run --release -p alsa_output --example delivery_spread -- [--bpm N] [--clock-pulses N] [--load N]` | Measures how late the ALSA queue delivers Timing Clock (F8) clock pulses (D-8) | Terminal |
| `cargo run --release -p alsa_output --example deadline_spread -- [--bpm N] [--clock-pulses N] [--load N] [--spin-us N]` | The same, with the program waiting for each clock pulse itself (D-8 fallback) | Terminal |

More on the measurements: `crates/alsa_output/examples/explainer.md`.

## Tools

| Command | What it does | Output |
|---|---|---|
| `cargo run --release -p atlas_tool` | Rebuilds the glyph atlas from `assets/fonts`. Run after changing the fonts or the glyph list | `assets/atlas/glyphs.pgm`, `assets/atlas/glyphs.txt` |
| `cargo run --release -p atlas_tool --example check_glyphs` | Checks the distance fields of a few glyphs | `scratch/atlas_check/` |

## Keeping this file current

When a program, example or control is added, changed or removed, update
this file in the same step.
