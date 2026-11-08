# song_notation_test

Revision 4 — 8 Nov 2026 (the song notation's new order, voice layers, nine mistakes)

## What it proves

That a song and a studio configuration written in the notation of
`docs/Architecture/Song Notation.md` (sections 4 to 6) load, and say what
was meant:

- `songs/Demo.song` loads and is printed track by track, section by
  section and bar by bar in the musician's terms ("bar 2 beat 2: a half
  note — chord ii7"). Reading the printout against the file shows that
  the shorthands work (a length left off is the one before it; `/` plays
  the chord or note before it), that the key and meter are right, and that
  a section's tracks with no line rest. A track with several voice layers
  in a section is printed layer by layer, each headed by its number, or
  its name when the track gives it one.
- `songs/Demo.studio` loads, and the studio covers the song: every track
  has its port and channel, every accent mark the song uses has a
  velocity, and every cue destination the song sends to is listed.
- Nine kinds of mistake each produce the one message EFS-6.3 asks for (the
  file, the line, what is wrong and what to type instead): a line for a
  track the song lacks, a chord on the mono bass, a first note with no
  length, a line in the old order (length first), chords with no key, no
  clock and transport track, a chord the notation does not know, a studio
  block with no port, and a duration without its unit. A file with a mistake loads nothing.
- A song track with no entry in the studio, and an accent mark the studio
  gives no velocity for, are caught by the coverage check; those two are in
  the unit tests of `studio_file.rs`.

The Demo's `Beat` track is an ordinary `mono_voice` that writes one pitch with
the accent where it is wanted (`| C4:q> / / / |`); there is no beat pulse
track type, and the studio gives it a port, a channel and velocities like
any voice.

It sends no MIDI. Playing the song is `song_play_test`, the next step.

## How to run

```
cargo run -p song_text --example song_notation_test
cargo run -p song_text --example song_notation_test -- path/to/other.song path/to/other.studio
```

Run it from the project folder. The exit status is 0 when both files load,
the studio covers the song and every built-in mistake gives its message.

## Findings

Not yet read by Jim.

Written while building it: a mistake in a track block or a section is
reported once, at the line of the mistake, and the block is then not checked
again, so one typo does not give a second message about the same block.
Chord names are shown as Roman numerals; the chord's notes (D minor
seventh, and where it sits on the staff) are worked out in `song_play_test`.

## Revisions

- 5 Nov 2026, revision 1: first version.
- Revision 2, 5 Nov 2026: the beat pulse is a mono voice; Demo files changed.
- Revision 3, 7 Nov 2026: places named in beats (and pulses in compound meters); tempo written in beats/min.
- Revision 4, 8 Nov 2026: song lines write what an element plays first, then its length (Song Notation Revision 15); voice layers printed; the mistake "a first note that says nothing" became "a first note with no length", and "a line in the old order" was added.
