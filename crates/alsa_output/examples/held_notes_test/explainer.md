# held_notes_test

Revision 1 — 12 Nov 2026

## What it proves

What a driven mono voice does when a second note on arrives while the
first note is still held, the way a keyboard player plays legato: whether
it keeps one gate and changes pitch, or gates again (Song Notation, Q34).

Each bar (4/4, default 120 q/min, four bars) the Bass port sends:

| Where | Message |
|---|---|
| beat 1 | note on 36 (channel 2, velocity 100) |
| beat 3 | note on 43, with 36 still held |
| end of the bar | note off 36 and note off 43, 5 ms before the bar ends |

The 5 ms is the lead the player gives any note that touches the next, so
the next bar's note on 36 gets a gate of its own. The `Beat pulse` port
sends steady quarter notes (note 60, channel 15, velocity 80, as in
`songs/Demo.studio`).

What to listen for on the Bass: one gate per bar, at beat 1, and a pitch
change at beat 3 with no new attack. A gate at beat 3 as well means the
receiver gates on every note on, held key or not.

This is not what the player sends today for a sample source: the player
ends the first note at the change (`legato_order_test`), so only one key
is held at a time. This test holds both keys.

The notes are made inside the program, not read from a song file; they are
the wire values of C1 and G1 as `songs/Legato_test.song` gives them through
the Demo studio. Change them with `--first-note` and `--second-note`.

## How to run

```
cargo run --release -p alsa_output --example held_notes_test -- [options]
```

Options are in the doc comment at the top of `main.rs`. The program opens
the ports `Bass` and `Beat pulse` in the client `Lead Sheet`, names them,
and waits for you to connect them (MIDI router or `aconnect`) and press
Enter. To see what ALSA delivers, start `aseqdump -p CLIENT:PORT` (the
numbers are printed, and `aconnect -l` lists them) before pressing Enter.

## Findings

12 Nov 2026: written; compiles and passes clippy.

12 Nov 2026, heard by Jim. Observed: FH-2 (gate and CV) into an analog
voice: hard gate at beat 1, legato pitch change at beat 3, gate held.
FH-2 CV and gate into the Disting NT's Macro Oscillator 2: only the first
note, no pitch change at beat 3. Poly FM on CV and gate: gate at beat 1
only, no pitch change. Under MIDI control the NT's Macro Oscillator 1 and
Poly FM gated at beats 1 and 3. Inferred, not tested: the NT algorithms
read pitch when the gate rises. Later the player's own sequence (`song_play_test` with `songs/Legato_test.song`) into Poly FM on CV and gate was legato at beat 3. Details in Song Notation, Q34.

## Revisions

- 12 Nov 2026, revision 1: first version; notes 36 and 43 (an octave below the first draft).
