# Live Lead Sheet — Song Notation Clarifications

Revision 3 — 11 Nov 2026 (closed: each question block is replaced by the
decision Jim made on 11 Nov 2026; brought into line with
`Song Notation.md`, Revision 20).

Claude raised these clarifications on 11 Nov 2026 about Jim's responses
in section 9, "Open questions", of `Song Notation.md` (Revision 18). Jim
answered them in this file and then in chat the same day. Nothing in
this file is open.

---

## Two chords in a bar, when a progression carries no timing

Decided (Jim, 11 Nov 2026):

- A progression carries no rhythm or timing, only the sequence of
  chords. The `@` spelling and Appendix A, "A chord's intended place
  inside a library bar", are withdrawn.
- The chords of a bar share the bar evenly. In 4/4, four chords last a
  quarter note each; in 3/4, two chords last a dotted quarter note each.
- The rhythm samples the harmony: each rhythm element takes the chord or
  note in effect at the moment it starts.
- A line may be coupled (`I:e. V`) or uncoupled. Uncoupled is a line
  marked `sample_source`, holding chords or notes with no lengths, and
  a rhythm line beneath it whose elements have a length and nothing
  before the colon (`:q`).
- The behaviour is a track and hold. Every rhythm element sounds, with a
  new gate at its start. If the source changes while the element is
  still sounding, the element changes to the new chord or note legato,
  with no new gate. A chord in the source is not skipped.
- Every source line has its own play head. Order of the work, all in the
  first version of the program:
  1. Coupled `.song` files play from text.
  2. `sample_source` and `dynamics` lines at the track's tempo, tested
     from text files.
  3. Separate play heads, so a source line can run at its own tempo,
     tested from text files.
  4. The GUI, dealing with several play heads from the outset.

## Octave annotations and the `home_octave` setting

Decided (Jim, 11 Nov 2026):

- The home octave comes from the clef: treble 4, alto 3, tenor 3,
  bass 2, grand 4.
- `+8va` and `-8va` stay in effect until changed. `loco` returns to the
  clef's octave.
- The track setting `home_octave` is removed.
- The `.studio` file's `octave_offset` is separate and stays.

## The base velocity

Decided (Jim, 11 Nov 2026):

- Normal is velocity 64, unless the `.studio` file gives the track its
  own normal velocity.
- The levels `pp`, `p`, `f` and `ff` and the marks `<<`, `<`, `>` and
  `>>` are relative to the normal velocity and shift with it. `f` and
  `>` are 3/8 of the way to velocity 127, `ff` and `>>` 3/4 of the way;
  `p` and `<` are 3/8 of the way to velocity 1, `pp` and `<<` 3/4 of
  the way. A mark on a note is measured from the level in force.
- A `.studio` entry may give an absolute velocity for `>` (`accent`)
  and for `>>` (`marcato`). `<` and `<<` have no word there and are
  always relative.
- There is no loudness curve in the song or the player, and the
  `.studio` file says nothing about how a device turns a velocity into
  loudness. That is the receiving device's business.

## Marks on a single note, and phrase dynamics

Decided (Jim, 11 Nov 2026):

- In the note line, a mark after a note changes that one note. `>` is
  an accent, `>>` is marcato, `<` and `<<` make the note softer. The
  backbeat `| q q> q q> |` is unchanged. `^` stays the fermata.
- Phrase dynamics go on their own `dynamics` line, drawn above the
  staff (or its highest ledger line) and below the chords.
- The levels are spelled with letters: `pp`, `p`, `mf`, `f`, `ff`. The
  wedges are only for gradual change: `<` for a crescendo, `>` for a
  diminuendo.
- A level holds until the next mark. "Return to normal" is written
  `mf`, which is velocity 64 when nothing else in the song or the
  `.studio` file alters it.
- The `dynamics` line is written in rhythm notation, like the cue line:
  each mark is an element with a length, and `r` is a stretch with no
  change. Example: `| p:w | < | f | mf |`.
- Each sounding element samples the `dynamics` line for its loudness.
  The `dynamics` line defaults to the track's tempo; it may later have
  its own tempo and play head, keeping the track's time signature.

## The same pitch in two voice layers

Decided (Jim, 11 Nov 2026):

- The effect is legato: one note, no second note on and no new gate.
  The note off is sent when the longest of the notes has ended.
- Legato is sent as overlapping notes: the next note on before the
  previous note off. MIDI 1.0 cannot change the pitch of a sounding
  note without a note on, so whether a new gate fires is the receiving
  device's setting.
- A device that cannot do legato from overlapping notes is a limitation
  of that device, and is accepted.

## Repeats

Decided (Jim, 11 Nov 2026):

- `/` repeats the previous note or chord (`I:q / / /`). There is no
  `..` sign.
- Repeat signs around a group of bars are written `|%n ... %|`. The
  number n is the total number of times the bars play: 2 means twice,
  3 means three times. With no number the bars play twice.
- A section's `repeat n` counts the same way.

## Functions

Decided (Jim, 11 Nov 2026):

- Functions are allowed in `.song` files. `trp` and the `fit` family
  (`fitQ`, `fitE`, `fitH`, `fitW`, `fitBar`) are already decided and
  built.
- Euclidean rhythms are added, with the usual two parameters (hits,
  steps). The steps fill the bar, as Strudel's steps fill the cycle.
- More capability can be added later as features.

## Beat grouping

Decided (Jim, 11 Nov 2026):

- A meter may state its grouping, e.g. `meter 7/8  grouping 2+2+3`.
- Grouping sets accents and beaming only; the beat stays the eighth
  note.
- With no grouping written, beat and pulse are identical.
