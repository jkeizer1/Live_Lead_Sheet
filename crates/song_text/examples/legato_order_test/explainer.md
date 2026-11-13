# legato_order_test

Revision 1 — 12 Nov 2026

## What it proves

What the player hands to the ALSA output, with no MIDI hardware, for a
song written with a sample source: each track's note on and note off
messages, in sending order, with the instant of each (milliseconds from the
song's first beat) and the channel as numbered on the device.

It checks one thing: where a sample source changes inside a sounding note,
the note on of the new note is sent first and the legato note off of the old
note follows it at the same instant. It prints "Legato check" for each track
with a legato note off, and exits with failure if one is not directly after
the note on that replaced it.

What it does not show: what ALSA does with two messages stamped with the
same instant, and what the receiver does with them. See "To see what ALSA
delivers".

## The song

`songs/Legato_test.song`, with `songs/Demo.studio`: the Bass plays one whole
note a bar, its source holding C2 for the first half of the bar and G2 for
the second; the Beat plays steady quarter notes. Two bars at 120 q/min.

Expected for the Bass (Demo studio: channel 2, one octave up, velocity 100):

| Instant | Message |
|---|---|
| 0 ms (bar 1 beat 1) | note on 48 |
| 1000 ms (bar 1 beat 3) | note on 55, then note off 48 at the same instant |
| 1995 ms | note off 55 (touching the next note on, 5 ms early) |
| 2000 ms (bar 2 beat 1) | note on 48 |
| 3000 ms (bar 2 beat 3) | note on 55, then note off 48 |
| 4000 ms | note off 55 |

## How to run

```
cargo run -p song_text --example legato_order_test
cargo run -p song_text --example legato_order_test -- path/to/other.song path/to/other.studio
```

Run from the project folder.

## To see what ALSA delivers

Play the same song (`song_play_test`, command in `songs/Legato_test.song`)
and in a second terminal watch what the program's Bass port sends:

```
aseqdump -p "Lead Sheet"
```

(`aseqdump` is part of alsa-utils; `aconnect -l` shows the port numbers if
the name is not accepted.) Look at beat 3 of each bar for the order of the
Note on and the Note off. This tells you whether ALSA keeps the order for
two messages with the same instant, which the legato relies on and which
has not been checked.

## Findings

12 Nov 2026: written; the listing above is what the player produced in the
sandbox, and the legato check passed (2 legato note offs, none misplaced).

12 Nov 2026, `aseqdump -p 130:2` while `song_play_test` played
`songs/Legato_test.song`: at each change ALSA delivered note on 55 before
note off 48, as the listing above says. Instants were not shown (no
timestamps in that output). The Disting NT still gates on every change.
Note that this sends one key held at a time (the first note ends at the
change), not two held keys: see `held_notes_test`.

## Revisions

- 12 Nov 2026, revision 1: first version; aseqdump finding added.
