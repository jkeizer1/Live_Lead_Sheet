# song_play_test

Revision 4 — 7 Nov 2026 ("count" replaced by beat)

## What it proves

That a composed song, written in the notation of
`docs/Architecture/Song Notation.md` and sent where a studio configuration
says, sounds as written: `songs/Demo.song` through `songs/Demo.studio`.

The player turns the song into events (`player::song_plan`): the repeats
are written out in play order (the count-in bar, then two passes of the
two-bar verse), each voice's notes are placed with their gate lengths and
their velocities from the studio, and the cues go to their destinations.
The schedule adds Timing Clock, and Start at the first position and Stop at
the end on the studio's clock port.

What to hear in the Demo:

- **Count-in:** one bar of four beats on the `Beat` track (channel 15,
  note C4 = 60), beat 1 at velocity 127 and the others at 64, then
  nothing else.
- **Keys** (channel 1): in C major, bar 1 four quarter notes of C major
  (C4 E4 G4); bar 2 a quarter rest, D minor seventh (D4 F4 A4 C5) for a
  half note on beat 2, G seventh (G4 B4 D5 F5) on beat 4.
- **Bass** (channel 2): C2 four times, then a rest, D2 for a half note and
  G2, each one octave lower on the wire because the studio gives the bass
  `octave_offset -1 octaves` (sounding C1, D1, G1).
- **Cues:** Start to the SQ-1 on each pass of the verse (its first bar).
- **Beat** continues through the verse, beat 1 of every bar accented
  (`>` in the song), as a mono voice like any other.

## Not applied yet (the program says so before playing)

- `settle_delay` and the legato mark have no effect yet.
- Tempo changes between sections: the program refuses a song whose
  sections have different tempi.

## How to run

```
cargo run --release -p song_text --example song_play_test
cargo run --release -p song_text --example song_play_test -- path/to/other.song path/to/other.studio
```

Run from the project folder. The program opens one output port for each
port in the studio (`Clock`, `Harmony`, `Bass`, `Beat pulse`, `To SQ-1`
for the Demo), names them, and waits for you to connect them (the MIDI
router or `aconnect`) and press Enter. The client is named `Lead Sheet`.
The sending loop is `../support/event_player.rs`, shared with
`rhythm_notation_test`.

## Findings

The unit and integration tests (`cargo test -p player -p song_text`)
check the notes, places and cues of the Demo against the score; the
sandbox has no ALSA sequencer, so the sending was not run where it was
written.

**5 Nov 2026, Bass only, to the Disting NT (raised two octaves on the
NT, so the Bass is audible).** The router's log of what `lead-sheet-bass`
sends matched the score exactly: C1 four times, a quarter rest, D1 as a
half note, G1, in the order note off then note on at each touching pair.
At the NT, a note that starts at the instant the previous note ends gets
no new gate (a pitch change, or silence for a repeated pitch); a note
after a rest gets its gate. Changing `retrigger_gap` from 2 ms to 350 ms
made no difference, as expected: the value is not applied (see above), so
that run says nothing about the gap.

`songs/Demo_bass.song`, with the same studio: the same Bass pitches as
eighth notes, each followed by an eighth rest, then four C notes the same
way. Every note had its gate and sounded as written. So the program, the
router and the NT's gate all work when notes are separated by a rest; what
fails is a note that touches the one before it. Not yet found: what the NT
receives at a touching pair (the order of the two messages, their timing),
and whether a gap of 2 ms is enough or a gap is needed at all.

Resolved by `alsa_output/examples/retrigger_gap_test`: the NT loses the
gate of a note that starts at the instant the previous one ends, and a
lead of 5 ms for the note off cures it. The program now sends the note off
of every touching note 5 ms early (more if the studio's `retrigger_gap` is
larger). To check: play `songs/Demo.song` about ten times and listen for a
Bass note without its gate. Not yet heard with this change.

## Revisions

- 5 Nov 2026, revision 1: first version.
- 5 Nov 2026, revision 2: findings of the Bass-only runs, including
  `songs/Demo_bass.song`.
- 5 Nov 2026, revision 3: the touching-note lead is applied; the Bass gate
  check is still to be heard.
- 7 Nov 2026, revision 4: "count" replaced by beat. The list "What to hear
  in the Demo" still describes the earlier Demo song, not the current one.
