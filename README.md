# Live Lead Sheet

A live lead sheet for improvising: it plays a song (chords, rhythm, bass)
to MIDI instruments and shows key, meter, chord names and voicings on a
staff, with the bars ahead in view. See `docs/Concept/Concept Brief.md`.

Revision 11 — 2026-11-23

## The program

```
cargo run --release -p app -- [SONG_FILE [STUDIO_FILE]]
```

Opens the song in the editor (the first draft; see `docs/Architecture/Editor
Draft.md`; every key at a glance in `docs/Architecture/Editor Cheat
Sheet.md`; what it cannot do yet in `docs/Architecture/missing_features_GUI.md`), with the libraries in `library/`. With no arguments it opens
`songs/Demo.song` and plays through `songs/Demo.studio`; a song given without
a studio opens with playing off. The terminal lists the MIDI output ports to
connect (MIDI router or `aconnect`); they stay open while the program runs.
The song file is only written by Ctrl+S; a change made to it in another
editor is loaded within a second when there are no unsaved edits here.

Five modes, in the order of the song's hierarchy, each render their own view,
open their own library tab and say where an edit goes; the header always
shows "Editing: …", what an edit changes now:

| Key | Mode | Works on | View |
|---|---|---|---|
| F1 | Arrange Sections | the composition: order, repeats, each entry's key and tempo, new sections | the composition as a list |
| F2 | Compose Sections | one section: its key and tempo, its lines' forms | the section's lines as lanes |
| F3 | Edit Lines | the music on a line, note by note | the staff |
| F4 | Song Harmony | the lead line's chords | every entry's bars with chord symbols and numerals |
| F5 | Track Harmony | one track's own harmony: a sampled line's source, or a chord track's chords | the staff, the selected bars shaded |

Each track's line is composed **coupled** (each note or chord with its own
length) or **sampled** (a sample source holding the pitches or chords, and a
rhythm line of `:q` elements that takes from it). The staff renders each in
its own form and labels it: a coupled line as one staff, a sampled line as
its source row (with the source's origin and play mode), its rhythm line,
and a staff of the derived notes. An applied rhythm plays a coupled line's own
notes or chords, or goes to a sampled line's rhythm line; an applied
progression goes onto a coupled line or into a sampled line's source. Only a
track with no line yet takes the lead's harmony. C (or a click on the track's
form label) changes the form, asking first.

| Control | Does |
|---|---|
| F1 to F5 | The mode (above) |
| Ctrl+`-` / Ctrl+`=` (or Ctrl+wheel) | Staff: narrower bars, more across / wider bars, fewer across. While editing, the selected entry starts at the left edge (a longer one turns whole pages from there); while playing the staff moves by half (the half shift) |
| Arrows | Move the selection: entry (F1, F4), bar (F2, F4, F5), line or track (F2, F3, F5), note (F3) |
| Shift+Left/Right | Widen or narrow the bar range |
| Shift+Up/Down | Move the selection a semitone (F1: the entry's key; F2: the section's key) |
| Ctrl+Up/Down | Move it a scale step: a note to the next scale note, a chord to the next degree's chord (F1, F2: a key by a fifth) |
| Ctrl+Shift+Up/Down | Move the selected note to the next tone of the lead's chord at that beat, up or down (F3) |
| Alt+Up/Down | Move notes an octave |
| K / Shift+K, F / Shift+F | Section key up/down a semitone, or by a fifth, composed notes moving with it (F1: the entry's own key) |
| T / Shift+T | Tempo up/down 5 beats/min (F1: the entry's own tempo) |
| [ / ] | One repeat fewer / more for the entry |
| M, D, Delete, Alt+Left/Right | Mute (repeat 0) or restore, duplicate, remove, move the entry |
| A | Accent, marcato, none on the selected note (F3) |
| R | The selected rest becomes a note of the same length (the root of the lead's chord there, on a bass; the chord, on a chord track), or a note becomes a rest (F3) |
| B | The selected mono track plays the roots of the lead's chords |
| C, C again | Change the selected track's line in the section between coupled and sampled; the first press asks. Every note sounds as before; a sampled line whose source changes while a note sounds (track and hold) stays sampled |
| N | A new section after the entry, from the progression under the library cursor |
| Tab | Between the main view and the library browser |
| Up/Down in the library (or the wheel over it) | Browse; each row is shown applied to the selected bars as a preview |
| Enter / Esc | Apply the preview / drop it (Esc also clears a bar range) |
| Space | Play from the selected entry, or stop; plays the preview when one is shown |
| Shift+Space, L | Play from the start; loop the selected entry |
| Ctrl+Z, Ctrl+Y | Undo, redo |
| Ctrl+S, Ctrl+Q | Save; quit (asks again when there are unsaved edits) |
| Click | Select an entry, bar, track, note, mode, tab or library row; double-click a library row applies it; click a track's form label to change it (twice) |

## Tests

Every song and studio that a test uses (the unit tests, the integration tests and
`song_notation_test`) is a file under `songs_canonical/`, in a folder named for
the test that reads it (`song_file/`, `studio_file/`, `sample_source/`,
`legato_mark/`, `note_spelling/`, `lilypond_export/`, `song_notation_test/`,
`count_in_and_verse/`, `chord_inversion/`, `layer_merge/`, `dynamics/`,
`octave_annotation/`, `tempo_change/`, `meter_control/`, `tempo_marks/`, `minor_key/`, `progression_library/`, `function_form/`, `composer/`). Each file starts with two comment lines: which test reads
it, what case it is, and for a bad file, what the loader must say. A test checks
exactly what its file says, so edit a copy in `songs/` to experiment, not the
canonical file.
The rhythm library tests (`rhythm_library.rs`, `rhythm_notation_test`) work the same
way, with `.library` files in `rhythm_library/` and `rhythm_notation_test/`.

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
| `cargo run --release -p render --example layout_preview -- [WIDTH HEIGHT] [hidden] [bar=N]` | Draws one frame of the spike on the CPU, without a window or GPU, to check the layout | `scratch/layout_preview.pgm` |
| `cargo run --release -p render --example screen_preview -- [WIDTH HEIGHT] [song=PATH] [STEP ...]` | Draws one frame of the editor on the CPU, in colour, without a window or GPU: `songs_canonical/composer/composer_demo.song` (or the song given) after steps such as `mode=3`, `convert`, `entry=3`, `bars=2`, `library`, `down=2`, `enter`, `right=3`, `tone=1`, `barwidth=-2`, `playing=TICKS` (listed in its doc comment) | `scratch/screen_preview.ppm` |
| `cargo run --release -p alsa_output --example control_track_test -- [options]` | Test: plays a count-in on the beat pulse, then a reset trigger and an SQ-1 Start on the music's first bar, with Timing Clock on its own port (D-10); optional SQ-1 restarts. Asks you to connect its ports, then waits for Enter. See its `explainer.md` | Terminal |
| `cargo run --release -p song_text --example rhythm_notation_test -- [LIBRARY_FILE] [--play "Name" [--bpm N] [--bars N] [--channel C] [--note N] [--transport]]` | Test: loads `library/Rhythms.library` (or the file given), prints every rhythm in beats and note values, and checks that six kinds of mistake are reported as EFS-6.3 asks. With `--play`, plays that rhythm on the beat-pulse port (Timing Clock on its own port) and asks you to connect the ports first; `--transport` adds Start and Stop, which the FH-2's clock output needs. See its `explainer.md` | Terminal |
| `cargo run -p song_text --example song_notation_test -- [SONG_FILE [STUDIO_FILE]]` | Test: loads `songs/Demo.song` and `songs/Demo.studio` (or the files given), prints the song track by track and bar by bar in beats and note values, checks that the studio gives every track and cue destination the song needs, and checks that eight kinds of mistake are reported as EFS-6.3 asks. See its `explainer.md` | Terminal |
| `cargo run --release -p song_text --example song_play_test -- [SONG_FILE [STUDIO_FILE]]` | Test: plays `songs/Demo.song` through `songs/Demo.studio` (or the files given): count-in, beat, chords, bass, the Start cue, with Timing Clock, Start and Stop on the studio's clock port. A note that ends where the next note on its track starts is sent 5 ms short (more with the studio's `retrigger_gap`). Where a sample source changes inside a sounding note, the new notes are sent first and the old ones released after them (legato), so the gate stays high on a mono receiver set to legato. The `_` mark does the same for a note and the next note on its track that starts where it ends; a marked note followed by the same pitch becomes one longer note. Opens one output port for each port the studio lists, names them and asks you to connect them first. To hear sample sources (chords and bass notes on their own lines, rhythm below): `-- songs/Sample_source.song songs/Demo.studio`. To hear the Nov 15 notations, one section each (7/8 grouping, chord inversions, repeat signs, `euclid`, the same pitch in two voice layers, the `dynamics` line, `+8va`/`-8va`/`loco`, tempo and meter changes between sections): `-- songs/Nov15_features_1_7.song songs/Nov15_features_1_7.studio`. To hear meter patterns (a `meter_control` line: 4/4 and 5/4 alternating, 3/4 3/4 4/4, 6/8 and 9/8, between plain-meter sections): `-- songs/Meter_control/Meter_control.song songs/Meter_control/Meter_control.studio`. To hear tempo marks on the lead line (a step, a carried tempo, a ramp that slows 120 to 60 beats/min over two bars, a repeated section that restarts at its own tempo): `-- songs/Tempo_marks/Tempo_marks.song songs/Tempo_marks/Tempo_marks.studio`. A tempo change between sections or at a tempo mark is played with a new anchor at its place; a ramp has one anchor at every clock pulse. Bar numbers in the output continue through meter changes. See its `explainer.md` | Terminal |
| `cargo run -p lilypond_export --example lilypond_export_test -- SONG_FILE [OUTPUT_FILE]` | Test: writes the song as a LilyPond file, to read it as sheet music; then `lilypond -o ~/tmp ~/tmp/NAME.ly` engraves it (LilyPond is installed separately: `sudo apt install lilypond`). Draws plain and dotted values and triplets; anything else is reported with its section, track and bar. See its `explainer.md` | `~/tmp/NAME.ly` (or OUTPUT_FILE), then `~/tmp/NAME.pdf` |
| `cargo run -p song_text --example legato_order_test -- [SONG_FILE [STUDIO_FILE]]` | Test: lists, for `songs/Legato_test.song` and `songs/Demo.studio` (or the files given), every note on and note off the player hands to the ALSA output, track by track, with its instant and channel, and checks that at a sample source change the new note on comes before the old note off. No MIDI hardware needed. To see what ALSA delivers, play the song with `song_play_test` and run `aseqdump -p "Lead Sheet"`. See its `explainer.md` | Terminal |
| `cargo run --release -p alsa_output --example held_notes_test -- [--bpm N] [--bars N] [--channel C] [--first-note N] [--second-note N] [--velocity V]` | Test: the most basic legato case as a keyboard sends it. Each bar the port `Bass` gets a note on at beat 1, a second note on at beat 3 with the first still held, then both note offs at the end of the bar; the port `Beat pulse` gets steady quarter notes. Finds out whether a mono voice keeps one gate and changes pitch at beat 3. Asks you to connect its ports, then waits for Enter. See its `explainer.md` | Terminal |
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
| `python3 tools/migrate_to_composition.py [--at-end] [--default-header] FILE.song ...` | Tool: moves a song written the old way (`repeat n` on sections, a section written again with only `repeat n`) to the `song_composition` block. The block goes before the first section (`--at-end`: at the end, keeping line numbers). Rewrites each file in place, so keep a copy; review the starting key, meter and tempo it picks. A file that already has a `song_composition` is left alone | The files given |
| `python3 tools/add_section_meters.py FILE.song ...` | Tool: writes a `meter` on every section that has none (a section no longer takes the meter of the one before it), copying the meter of the nearest section above, else the song_composition header's. Run it after `migrate_to_composition.py`. Rewrites each file in place, so keep a copy. A section with a `meter_control` line is left alone | The files given |
| `./play_here.sh` (in a song folder: copy `play_here.sh` there, or link to it with `ln -s ~/Documents/Live_Lead_Sheet/play_here.sh .`) | Tool: plays the newest `.song` and the newest `.studio` file in the folder it is run from, with `song_play_test` (as the `cargo run --release -p song_text --example song_play_test -- SONG STUDIO` line does), from any folder. Names the file chosen when there was a choice, and says so if the folder has none of either. A copy finds the project at `~/Documents/Live_Lead_Sheet`, or at `LIVE_LEAD_SHEET_DIR` if set | Terminal; stays open until you press Enter |
| `./tune_delay.sh [--runs N] [--dry-run] [--set-ms MS] [--port-skipped NAME]` (in a song folder: copy `tune_delay.sh` there, or link to it as for `play_here.sh`) | Tool, specific to Jim's studio: with audio flowing from the NT through the PC to the L-12, measures that path's delay N times (default 5; the measuring code is `~/Documents/LinuxAudioNotes/audio_state_2.py`, or `AUDIO_STATE_FILE`), takes the median in whole milliseconds, and sets `delay_trigger MS ms` in the newest `.studio` file of the folder on every track whose port is not `DistingNT`, except a track with no channel (the clock). `--dry-run` shows the change and writes nothing; `--set-ms` skips the measuring. The logic is `tools/tune_delay.py` | Terminal; the studio file |

## Keeping this file current

When a program, example or control is added, changed or removed, update
this file in the same step.
