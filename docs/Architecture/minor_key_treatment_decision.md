# Live Lead Sheet — Minor key treatment: decision points

Revision 1 — 8 Nov 2026

**Status: framing only, nothing decided.** Minor keys are Song Notation
Q12. Today `key C minor` is refused: `Mode` in `crates/vocabulary/src/key.rs`
has only `Major`, and the plain voicing counts degrees from the major
scale (`semitones_above_tonic` in `crates/player/src/voicing.rs`). This
note lists the decisions minor keys need, so they can be taken one at a
time. Found while writing BWV 610 (C minor), which loads only with its
key changed.

## Background: where the three minor forms differ

In C minor the natural, harmonic and melodic (ascending) forms agree on
degrees 1 to 5 and differ only on degrees 6 and 7:

| Degree | Natural minor | Harmonic minor | Melodic minor (ascending) |
|---|---|---|---|
| 6 | A♭ | A♭ | A |
| 7 | B♭ | B | B |

So `i`, `iiø7`, `III`, `iv` and `V7` mean the same in every form. A chord's
quality is already explicit in its name (case and suffix): `V7` is a
dominant seventh, G B D F, without any rule about which minor is meant.

## Decision points

**1. What a numeral's root is in a minor key.** Two established
conventions:

- *Classical (common practice).* Numerals count from the minor tonic; the
  chord's quality tells which form is meant: `VII` is B♭ major (the
  subtonic), `vii°` is B diminished (the leading-tone chord). Read
  naturally by musicians; the root then depends on the quality, a rule the
  program must be given.
- *Pop and jazz (major-referenced).* Numerals always count from the major
  scale on the same tonic, and lowered degrees are written with a flat:
  `i bIII iv v bVI bVII`. Unambiguous, and what the parser and voicing
  already do; how most lead sheets and progression books write minor
  (`i bVI bVII`, minor blues `i7 iv7 i7 bVI7 V7`).

There may be further variants (for example, which form a library entry or
a section declares). This decision fixes what `III`, `VI` and `VII` mean in
every minor song and library entry from then on.

**2. Key signature and note spelling.** A minor key takes its relative
major's signature (C minor: three flats). Affected: the LilyPond view
(`\key c \minor`), note spelling's choice of flats or sharps for chord
notes, and later the renderer. Open: how raised 6th and 7th degrees are
shown (accidentals in the music against the signature).

**3. The key wheel (EFS-3.7).** Its segments and function colours depend
on decision 1. Open: one kind of wheel or two (a minor wheel of its own,
or the relative major's wheel turned to another tonic), and how the
function colours (tonic, subdominant, dominant) apply in minor.

**4. Modes.** Q12 groups minor with modes and how a change of mode is
shown. Open: whether minor is defined as one mode (Aeolian) plus the
raised 6th and 7th, ready for other modes later, or as a second fixed
case beside major. This shapes the `Mode` type in `key.rs`.

**5. The libraries.** The starter progression library lists a minor blues
and a minor ii–V–i (EFS-7.1); entries carry `mode major` today. How minor
entries are written follows decision 1, and whether an entry names its
minor form is open.

## Where the code changes, once decided

`key.rs` (`Mode`), `contents_text.rs` (`parse_key`), `voicing.rs` (degree
roots in minor), `note_spelling` (chord-note spelling, flats or sharps),
`lilypond_export` (`\key … \minor`), the key wheel in `render`, and the
progression library format.
