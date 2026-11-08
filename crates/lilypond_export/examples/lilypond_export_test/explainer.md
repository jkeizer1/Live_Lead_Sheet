# lilypond_export_test

Revision 2 — 8 Nov 2026 (findings: Demo corrected, BWV 610 with its sharps)

## What it proves

That a song file can be read as sheet music. It loads a song, writes it
as a LilyPond file, and the `lilypond` program engraves that file as a
PDF. Reading the PDF against the song file shows what the song really
plays, which is quicker and less tiring than reading the text.

The written notes come from the `note_spelling` crate, which works out,
once for every converter, which note value and dots each element is, which
elements form a triplet, and which written pitches a chord's notes are
(the player's plain voicing, each note spelled by its place in the chord:
in C major `ii7` is D F A C). The `lilypond_export` crate only writes
LilyPond's words for them.

What the page shows:

- each voice track as a staff in its clef; a grand-staff track as two
  braced staves, each voice layer on the upper or lower staff by its
  median pitch (middle C and above going up);
- voice layers sharing a staff with stems up and down;
- each section's name as a boxed rehearsal mark, its key, meter and tempo
  (in the meter's beats) where they change, and `repeat n` as repeat bars;
- a track with no line in a section resting for its bars;
- the lead track's chords as Roman numerals above voice layer 1, where the
  chord changes;
- a cue control track as a one-line rhythm staff with each cue's name;
- accents, marcato, staccato, fermatas, ties, and legato as slurs.

Limitations, by design for now:

- Only plain and dotted values and triplets are drawn. A `fit` group such
  as five in the time of a quarter stops the export with a message naming
  the section, track and bar; the song still plays.
- Layout choices are LilyPond's own (line breaks, stem directions within a
  voice, beaming). A voice layer stays on one staff of a grand staff even
  where the score moves an inner voice to the other staff.
- Nothing is sent and nothing in the song changes.

## How to run

```
cargo run -p lilypond_export --example lilypond_export_test -- songs/Demo.song
lilypond -o ~/tmp ~/tmp/Demo.ly
```

The file goes to `~/tmp/<song file name>.ly` unless a second argument gives
another path; `~/tmp` must exist. LilyPond is not part of the program:
install it with `sudo apt install lilypond` (Linux Mint's package is
version 2.24, which the file is written for). The PDF is
`~/tmp/Demo.pdf`.

## Findings

8 Nov 2026, written while building it (LilyPond 2.25.12 in Claude's
workspace; not yet run on Jim's machine):

- All five songs in `songs/` engrave with no LilyPond warning.
- `songs/Demo.song`: the Keys line wrote `VI`, an upper case numeral,
  which is A major (A C♯ E), where the comment above the section says the
  progression is C – G – Am – F. The page showed the C♯, and the player
  sent it. Jim corrected it to `vi` (A minor) the same day.
- BWV 610 (a scratch copy in E♭ major, since minor keys are still an open
  question in Song Notation, Q12) engraves with the chorale and the alto
  on the upper staff, stems up and down, the tenor on the lower staff and
  the pedal on its own, and matches the Mutopia edition bar for bar apart
  from the key. Its F sharps read correctly once `#` stopped starting a
  comment in the middle of a bar (Song Notation, Q22).

## Revisions

- 8 Nov 2026, revision 1: first version.
- 8 Nov 2026, revision 2: findings updated after Demo's `vi` and the comment rule.
