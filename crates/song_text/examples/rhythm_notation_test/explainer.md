# rhythm_notation_test

Revision 5 — 7 Nov 2026 (places are named in beats; "count" replaced by beat)

## What it proves

That the rhythm notation of `docs/Architecture/Song Notation.md`
(section 3) is read exactly:

- the starter library `library/Rhythms.library` loads, and every rhythm is
  printed in the musician's terms ("bar 1 beat 2: an eighth note,
  accent"), grouped by meter, then group, so the notation can
  be checked by eye against what was meant;
- six kinds of mistake each produce the one message EFS-6.3 asks for (the
  file, the line, what is wrong and what to type instead), and a file with
  a mistake loads nothing.

With `--play`, it then plays the chosen rhythm on the beat-pulse port at a
chosen tempo, repeated to fill the bars asked for, with Timing Clock on its
own port, for listening on the FH-2. The rhythm is played as a mono voice
that plays one fixed note (the "beat pulse"), like any voice: a note begins
at each sounding element that no tie leads into, a tied note is one note
as long as the elements joined, and the note's written length is its gate
length. The rhythm's own marks set the velocity: `>` and `>>` send 127,
a note with no mark sends 64. (Revision 2 gave beat 1 of each bar the
accent whatever the rhythm said, and cut every pulse to a sixteenth; both
special cases are gone. The starter library now writes the beat 1 accent
with `>` in the pulses, Charleston and the bossa comp.)

## Setup on Jim's studio

Three things together make the configuration (all three are needed):

- **The router configuration** `rhythm_test.toml`, in the MIDI Router
  project's `examples/` folder. It has the two routes from this program
  (`Clock`: clock and transport; `Beat pulse`: notes on channel 15, to the
  FH-2) and a route from `Midi Through` that carries system-exclusive to
  the FH-2, so the FH-2 can be configured through the router.
- **The FH-2 configuration** `rhytm_notation_test.syx`, in this folder (the
  file name has a typo; rename it when convenient): the beat pulse on FH-2
  output 1 and the clock on output 5.
- **The Disting NT preset** `N5Test`, the NT's side of the same setup.

## How to run

```
cargo run -p song_text --example rhythm_notation_test
cargo run -p song_text --example rhythm_notation_test -- path/to/other.library
```

To play (the program names its two ports and waits for you to connect
them, as `control_track_test` does, then for Enter):

```
cargo run --release -p song_text --example rhythm_notation_test -- --play "Charleston"
cargo run --release -p song_text --example rhythm_notation_test -- --play "Anticipation" --bpm 90 --bars 4
```

Options (`--bpm`, `--bars`, `--channel`, `--note`, `--transport`) are in the doc comment at
the top of `main.rs`. Ports: `Clock` (Timing Clock) and `Beat pulse`
(channel 15, note 60 unless changed). The client is named `Lead Sheet`.
Use `--release` when playing, as for the other timing programs.

The FH-2's clock output pulses only between a Start and a Stop (FH-2
Observations, 2 Nov 2026), and the beat pulse needs no transport, so the
test sends Timing Clock alone unless you add `--transport`, which sends
Start at the first note and Stop at the end on the `Clock` port. Add it to
see the FH-2's clock output (e.g. jack 5).

What to listen and watch for, on the FH-2 gate (or a voice it drives):

- *Charleston*: a gate on beat 1 (the louder, accented one) held for a
  dotted quarter, and one on the "and" of beat 2 held for an eighth, then
  silence for the half-note rest, every bar.
- *Anticipation*: one gate every two bars, on the "and" of beat 4. The
  tied quarter after the barline must not retrigger.
- *Quarter-note triplets*: six evenly spaced gates in the bar, none on
  beat 2 or 4.
- *Backbeat*: beats 2 and 4 are louder (its `>` marks); beat 1 is
  normal.
- Each gate lasts the written note value: a quarter note is held for a
  quarter note, an eighth for an eighth. On a short gate, such as an
  eighth note, an FH-2 output set to CV and gate should still drop
  between notes.

Run from the project folder. The exit status is 0 when the library loads
and every built-in mistake is reported as expected. Debug is right: this is
a test, not a measurement.

## Findings

*5 Nov 2026.* The starter library (10 rhythms) loads. Running it showed
three faults, now fixed, each now covered by a unit test:

- A meter line with a mistake (`meter 4/3`) also produced "the bars come
  before the meter" for the bar line after it. Only the first is reported.
- The tie error named a place in ticks. It now says "bar 1 count 4 plus an
  eighth note".
- A triplet quarter note read "2/3 of a quarter note". It now reads "a
  triplet quarter note".

*7 Nov 2026.* Places are now named in beats, as the counter shows them,
with the pulse added in compound meters: the tie error says "bar 1 beat
4". A place between pulses is named by the pulse it falls in.

Decided in passing: `r` and the dots may stand in either order after the
note letter, so `hr.` and `h.r` are the same dotted half rest (Song Notation
used both spellings; revision 7 says so).

Unit tests (`cargo test -p song_text -p vocabulary`) cover the parts: dots
(including the half ticks of a double-dotted thirty-second note), `trp` and
`fit` groups (an even seven over a bar; eleven over a bar rounds once and
still fills the bar exactly), marks, ties and the error messages.

*5 Nov 2026, first listening (Jim, FH-2, router).* Beat pulse: the accent
on count 1 of Charleston was heard. The router's own clock log showed
Timing Clock at a steady 120.0 BPM (48 pulses/s, 20.0–21.0 ms apart),
passed to the FH-2, and the run lasted 16 s, as 8 bars at 120 q/min should
(timing of the notes against the clock has not been measured). The FH-2's
clock output on jack 5 showed nothing: expected, since this run sent no
Start (see `--transport`; rerun with it to confirm).

*5 Nov 2026, third listening (Jim): the other rhythms sounded right, but
`Basic 6/8 pulse` was wrong. It was written `| q. q. |`, two dotted quarters,
so the program sent a pulse on count 1 (accented) and count 4 only (in 6/8
the counts are eighths), and Jim expected one pulse per count. The player
did what was written; the starter rhythm was the mistake. It is now six
eighths, `| e e e e e e |`, and the old one is kept as `6/8 two-beat`.
Also found: a rhythm name typed with a trailing space was not found;
the name is now trimmed.

*5 Nov 2026, fourth listening (Jim): `Basic 6/8 pulse` now sounds right,
the accent on count 1 followed by counts 2 to 6. Rhythms heard so far:
Charleston and the others Jim tried, and this one; the tie of Anticipation
was not singled out in his reports.*

*5 Nov 2026, second listening (Jim), `--play "Charleston" --transport`.*
The FH-2's clock output (jack 5) pulsed, and the clock and the beat pulse
were in time with each other. This confirms that the clock output needs a
Start, and that the schedule's Start before the first Timing Clock works
through the router. Only Charleston was tried, and the timing was judged by
ear and eye, not measured; the other rhythms (Anticipation's tie,
the triplets) have not been heard yet.

*Before that, 5 Nov 2026, not yet heard.* The play half was built and its schedule is
unit-tested (repeating a rhythm, joining tied notes, accent on the bar's
first beat, refusing a rhythm in another meter), but it could not be run
where it was written: the sandbox has no ALSA sequencer
(`/dev/snd/seq`). The first listening result goes here.

*5 Nov 2026, listening after the beat pulse became a mono voice (Jim, FH-2
and Disting NT, the NT set to velocity gates).* Charleston, the
quarter-note pulses (touching notes) and the eighth-note pulses all sounded
correct and the timing was very accurate. Touching notes of equal velocity
retriggered the NT's velocity gate with no retrigger gap, so Q11 is not
needed for that case. Judged by ear; not measured.

The play half uses the sending loop in `../support/event_player.rs`, shared
with `song_play_test`; it is a trimmed copy of the loop of
`alsa_output/examples/control_track_test`. Its proper home is the app's
sending thread (Architecture, Next, step 3); the shared module goes when
that exists. Revision 4 moved the play half onto the shared module, so it
needs one more listening.

## Revisions

- Revision 1, 5 Nov 2026: print the library, check six mistakes.
- Revision 2, 5 Nov 2026: `--play` plays a rhythm on the beat-pulse port; `--transport` adds Start and Stop; setup notes; 6/8 rhythm corrected; name trimmed.
- Revision 3, 5 Nov 2026: the beat pulse is a mono voice; velocity comes from the rhythm's marks, gate length from the written length; library accents written with `>`.
- Revision 4, 5 Nov 2026: the play half uses the shared `support/event_player.rs`; listening findings with the NT's velocity gates.
- Revision 5, 7 Nov 2026: places named in beats (and pulses in compound meters); "count" replaced by beat.
