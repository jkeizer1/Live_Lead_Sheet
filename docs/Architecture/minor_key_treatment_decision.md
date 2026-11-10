# Live Lead Sheet — Minor key treatment: decision points

Revision 13 — 9 Nov 2026 (cross-references to Song Notation's Q12 removed from items 1 and 4)

**Status: items 1 to 3 decided (9 Nov 2026); items 4 and 5 have OPEN_QUESTIONs (search for OPEN_QUESTION).** Minor keys are Song Notation
Q12. Today `key C minor` is refused: `Mode` in `crates/vocabulary/src/key.rs`
has only `Major`, and the plain voicing counts degrees from the major
scale (`semitones_above_tonic` in `crates/player/src/voicing.rs`). This
note lists the decisions minor keys need, so they can be taken one at a
time. Found while writing BWV 610 (C minor), which loads only with its
key changed.

**Jim's: Comment**
- The BWV 610 score is written in E flat Major;
- the relative minor is C minor.
- The lilypond file indicates C minor (unkwown to me as to why)
```
global = {
   \key c \minor
   \time 4/4
   \tempo Largo
   \accidentalStyle default
}
```

**Answer (9 Nov 2026):** three flats is the signature of both E♭ major
and its relative minor, C minor (the minor on E♭ major's 6th degree), so
the signature alone does not tell the key; the music does. BWV 610 is in
C minor, and the LilyPond file is right:

- the tonic is C: the pedal starts on C, and the first phrase and the
  whole piece end on C;
- B naturals (LilyPond's `h`) appear throughout, the leading tone of C
  minor; in E♭ major the B would be B♭. Bar 3's soprano rises G A♮ B♮ C,
  the ascending melodic minor.

So the song needs `key C minor`, and minor keys must be supported.
`key Eb major` plays BWV 610's notes correctly, since it has no chord
numerals and notes are absolute pitches, but it misstates the key: the
key wheel and any numerals would centre on E♭, making the tonic chord
`vi` instead of `i`. It is a stopgap, not the song's key.


## Reference: the three minor scales, and the major scale

The three minor scales are fixed by their intervals; no practice reads
them differently. Tables for C minor, with each step's distance from the
step before and from the tonic in semitones (a half step is 1 semitone,
a whole step 2).

**Natural minor.** Steps: whole, half, whole, whole, half, whole, whole.

| Step | Note | Interval above the tonic | From the previous step | From the tonic |
|---|---|---|---|---|
| 1 | C | unison | — | 0 semitones |
| 2 | D | major 2nd | whole step, 2 semitones | 2 semitones |
| 3 | E♭ | minor 3rd | half step, 1 semitone | 3 semitones |
| 4 | F | perfect 4th | whole step, 2 semitones | 5 semitones |
| 5 | G | perfect 5th | whole step, 2 semitones | 7 semitones |
| 6 | A♭ | minor 6th | half step, 1 semitone | 8 semitones |
| 7 | B♭ | minor 7th | whole step, 2 semitones | 10 semitones |
| 8 | C | octave | whole step, 2 semitones | 12 semitones |

**Harmonic minor.** Steps: whole, half, whole, whole, half, one and a
half, half. The 7th is raised a semitone (the leading tone).

| Step | Note | Interval above the tonic | From the previous step | From the tonic |
|---|---|---|---|---|
| 1 | C | unison | — | 0 semitones |
| 2 | D | major 2nd | whole step, 2 semitones | 2 semitones |
| 3 | E♭ | minor 3rd | half step, 1 semitone | 3 semitones |
| 4 | F | perfect 4th | whole step, 2 semitones | 5 semitones |
| 5 | G | perfect 5th | whole step, 2 semitones | 7 semitones |
| 6 | A♭ | minor 6th | half step, 1 semitone | 8 semitones |
| 7 | B | major 7th | augmented 2nd, 3 semitones | 11 semitones |
| 8 | C | octave | half step, 1 semitone | 12 semitones |

**Melodic minor (ascending).** Steps: whole, half, whole, whole, whole,
whole, half. The 6th and 7th are raised a semitone. In classical
teaching it descends as the natural minor; in jazz it is used the same
both ways (often called jazz minor). This concerns melody, not chord
names.

| Step | Note | Interval above the tonic | From the previous step | From the tonic |
|---|---|---|---|---|
| 1 | C | unison | — | 0 semitones |
| 2 | D | major 2nd | whole step, 2 semitones | 2 semitones |
| 3 | E♭ | minor 3rd | half step, 1 semitone | 3 semitones |
| 4 | F | perfect 4th | whole step, 2 semitones | 5 semitones |
| 5 | G | perfect 5th | whole step, 2 semitones | 7 semitones |
| 6 | A | major 6th | whole step, 2 semitones | 9 semitones |
| 7 | B | major 7th | whole step, 2 semitones | 11 semitones |
| 8 | C | octave | half step, 1 semitone | 12 semitones |

**Major, on the same tonic** (for comparison; Practice B below counts
from it). Steps: whole, whole, half, whole, whole, whole, half.

| Step | Note | Interval above the tonic | From the previous step | From the tonic |
|---|---|---|---|---|
| 1 | C | unison | — | 0 semitones |
| 2 | D | major 2nd | whole step, 2 semitones | 2 semitones |
| 3 | E | major 3rd | whole step, 2 semitones | 4 semitones |
| 4 | F | perfect 4th | half step, 1 semitone | 5 semitones |
| 5 | G | perfect 5th | whole step, 2 semitones | 7 semitones |
| 6 | A | major 6th | whole step, 2 semitones | 9 semitones |
| 7 | B | major 7th | whole step, 2 semitones | 11 semitones |
| 8 | C | octave | half step, 1 semitone | 12 semitones |

The three minor scales agree on steps 1 to 5 and differ only on steps 6
and 7. A chord's quality is already explicit in its name (case and
suffix): `V7` is a dominant seventh, G B D F, whichever minor scale is
in use.

## Decision points

**1. What a numeral's root is in a minor key.**

*Decided (Jim, 9 Nov 2026):* each song declares its practice once for
the whole song; a song does not change practice partway. It is written:

```
chord_numeral_standard key_dependent
chord_numeral_standard major_referential
```

| Value | Practice | Counts a numeral's step in | In C minor, the E♭ chord is |
|---|---|---|---|
| `key_dependent` | A, classical | the key's own scale (natural minor, Dorian and so on) | `III` |
| `major_referential` | B, major-referenced | the major scale on the key's tonic | `bIII` |

Both count from the key's tonic; they differ only in which scale. In a
major key they agree.

**The issue to decide: how the program finds a chord's root in a minor
key.** In a minor song, the same numeral can mean two different root
notes, depending on the practice it is written in. Everything else about
the chord (its quality and how its notes are laid out) is the same either
way. Once the root is fixed, the program can play the chord.

A Roman numeral gives a
chord's root as a step of a scale on the tonic; case and suffix give its
quality (upper case major, lower case minor, `°` diminished, `7` a
seventh). In a major key there is one scale to count steps in, so a
numeral means one thing. In a minor key there are three scales (the
reference tables above), and two practices for naming chords are in
established use. They come from different traditions:

- **Practice A, classical (common-practice harmony).** How harmony
  textbooks and analyses of Bach, Mozart and their contemporaries name
  chords. It describes minor-key music as written in that tradition,
  where the 6th and 7th steps move between the minor scales (lowered in
  the natural minor, raised in the harmonic and melodic minor, toward
  the tonic). So a numeral counts in the key's own natural minor scale,
  and the chord's quality shows when a raised step is the root.
- **Practice B, major-referenced.** How pop and jazz lead sheets,
  progression books and chord software name chords. It measures every
  key, major or minor, against one fixed yardstick, the major scale on
  the tonic, so a numeral always means the same interval above the
  tonic, and chords on lowered steps carry a flat. This makes
  progressions comparable across keys and modes: `bVII` is the same
  chord in C major and in C minor.

The difference between them is only where a chord's root is looked up.
The numeral's number is a step (`VI` is step 6); its root is that step's
note in one of the reference tables above, and the two practices use
different tables:

**Key for all examples: C minor.**

*What changes and what does not.*

- **Chord quality** (major, minor, diminished, a seventh): given by the
  numeral's case and suffix. The same in both practices: `VI` is a major
  chord either way.
- **Voicing**: how a chord's notes are laid out (which octave each note
  is in, which note is lowest). In the program, `plain_voicing` in
  `voicing.rs` turns a chord name into the notes the player sends: it
  finds the root, adds the intervals of the chord's quality, and places
  the notes in close position above the track's home octave.
- **The practices change only the first of those steps: which root is
  found.**

| `VI` in C minor | Root | Quality (from the numeral) | Notes sent |
|---|---|---|---|
| Practice A | A♭ | major | A♭ C E♭ |
| Practice B | A | major | A C♯ E |

*The two practices side by side.*

| | Practice A: classical | Practice B: major-referenced | For our program, if A | For our program, if B |
|---|---|---|---|---|
| Root of `VI` (step 6) | **VI root: A♭** (minor 6th) | **VI root: A** (major 6th) | the voicing needs a natural minor table beside the major one | no change: the voicing already uses the major table |
| `III`, `VI`, `VII` in C minor | E♭, A♭, B♭ major | E, A, B major | | |
| The same chords on E♭, A♭, B♭ are written | `III`, `VI`, `VII` | `bIII`, `bVI`, `bVII` | | the flats already work |
| A root on a raised 6th or 7th (harmonic, melodic minor) | shown by the chord's quality, or a ♯ or ♮ before the numeral: `vii°` is B, `VII` is B♭ | written plainly: `vii°` is B, `vi°` is A | the root depends on the quality: a rule to write and test, and a mistyped case changes the root silently | nothing special |
| Example: the Andalusian cadence (Cm B♭ A♭ G) | `i VII VI V` | `i bVII bVI V` | | |
| Example: a minor blues (Cm7 Fm7 Cm7 A♭7 G7) | `i7 iv7 i7 VI7 V7` | `i7 iv7 i7 bVI7 V7` | library entries written this way | library entries written this way |
| The key wheel's labels in C minor | `i ii° III iv v VI VII` | `i ii° bIII iv v bVI bVII` | | flats on three segments |
| Where you meet it | harmony textbooks; analyses of Bach chorales | lead sheets, jazz, progression books, chord tools in software | matches what you read in classical analysis | matches lead sheets and progression books |

*The same chords, both ways (C minor).*

| Chord sounding | Practice A writes | Practice B writes |
|---|---|---|
| C minor | `i` | `i` |
| D diminished | `ii°` | `ii°` |
| E♭ major | `III` | `bIII` |
| F minor | `iv` | `iv` |
| G major, G7 | `V`, `V7` | `V`, `V7` |
| A♭ major | `VI` | `bVI` |
| B♭ major | `VII` | `bVII` |
| B diminished | `vii°` | `vii°` |

The two practices agree on every chord whose root is step 1, 2, 4 or 5,
and on the leading-tone chord. They differ only for roots a minor 3rd,
minor 6th or minor 7th above the tonic: A writes them plain, B with a
flat.

*Where the two conflict:* the same text plays different notes.

| Written in C minor | Practice A plays | Practice B plays |
|---|---|---|
| `III` | E♭ major (E♭ G B♭) | E major (E G♯ B) |
| `VI` | A♭ major (A♭ C E♭) | A major (A C♯ E) |
| `VII` | B♭ major (B♭ D F) | B major (B D♯ F♯) |

- A song or library entry written in one practice and read in the other
  plays wrong chords with no error, the same kind of silent difference
  as Demo's `VI` and `vi` in C major.
- So whichever practice is chosen, the notation must state it, and a
  minor song can be read one way only.

This decision fixes what `III`, `VI` and `VII` mean in every minor song
and library entry from then on.

**2. Key signature and note spelling.** *Decided (Jim, 9 Nov 2026):*
nothing to settle. The signature follows from the declared tonic and
mode; raised 6th and 7th steps are written as accidentals. A minor key takes its relative
major's signature (C minor: three flats). Affected: the LilyPond view
(`\key c \minor`), note spelling's choice of flats or sharps for chord
notes, and later the renderer.

**3. The key wheel (EFS-3.7).** *Decided (Jim, 9 Nov 2026): dropped.*
The EFS (EFS-3.7) and the Concept Brief still describe it and are to be
updated.

**4. Modes.** How a change of mode is shown in a song, and whether it
changes the numerals.

*Modes: background.*

- **What a mode is.** A mode is the major scale's pattern started on one
  of its steps. With C major as the parent:

  | Mode | Starts on step | Notes | Other name |
  |---|---|---|---|
  | Ionian | 1 | C D E F G A B | the major scale |
  | Dorian | 2 | D E F G A B C | |
  | Phrygian | 3 | E F G A B C D | |
  | Lydian | 4 | F G A B C D E | |
  | Mixolydian | 5 | G A B C D E F | |
  | Aeolian | 6 | A B C D E F G | the natural minor scale |
  | Locrian | 7 | B C D E F G A | |

- **Parent.** A mode's parent is the major scale whose notes it uses:
  C major is the parent of D Dorian and of A minor; E♭ major is the
  parent of C minor.
- **A key is a tonic plus a mode.** A mode is named by its own tonic, not
  its parent: D Dorian, not "C major Dorian". A mode name alone is only
  an interval pattern and is undefined: Lydian could be F Lydian
  (F G A B C D E) or D Lydian (D E F♯ G♯ A B C♯).
- **Key signature.** A mode is usually written with its parent's
  signature: D Dorian with none, like C major. A less common practice
  writes the tonic's minor or major signature with accidentals (D Dorian
  with D minor's one flat, and B♮ where it occurs).
- **A signature never identifies the key.** Three flats fit E♭ major,
  C minor, F Dorian and the other modes of E♭ major. The song's
  declaration (`key C minor`) states which is meant.
- **Numerals restart at the mode's tonic**, and chord qualities follow
  the mode's intervals: `ii` of C Ionian is D minor; `ii` of D Dorian is
  E minor.
- **The two practices of decision 1 apply to modes the same way.** In
  D Dorian:

  | | Practice A (classical) | Practice B (major-referenced) |
  |---|---|---|
  | Counts steps in | D Dorian's own scale | D major |
  | The chords written | `i ii III IV v vi° VII` | `i ii bIII IV v vi° bVII` |

- **Minor and modes.** The natural minor scale is the Aeolian mode. The
  harmonic and melodic minor scales are not modes of the major scale;
  they have modes of their own, used in jazz (Phrygian dominant is the
  harmonic minor started on its 5th step).

- *Proposed direction (Jim, 9 Nov 2026).*
  - Common practice numbers chords from wherever the music's tonal
    centre is:

    | The music | Usually numbered | The mode is |
    |---|---|---|
    | stays centred on C, with a Dorian colour now and then | from C: E minor is `iii` | an annotation for the reader |
    | moves its centre to D (phrases rest on D, a D Dorian vamp) | from D: E minor is `ii`, the characteristic major chord `IV` | a declaration the program uses |

  - A song-level switch, declared once like the practice (decision 1),
    says which applies:
    - *off:* a mode marking is an annotation only, shown on the playing
      screen, in exported music and on printed scores; numerals count
      from the declared key;
    - *on:* a mode marking is a declaration: numerals count from that
      mode's tonic until the next marking.
  - A mode marking works like a key change, with the same boundary rule:
    it is set per section and changes only at a section boundary.
  - Modes of the harmonic and melodic minor scales (such as Phrygian
    dominant) are out of scope for now.

**OPEN_QUESTION:** Is the proposed direction for modes decided?
- Context: the direction above (numbering by tonal centre; a song-level
  switch making mode markings annotations or declarations; markings set
  per section like key changes) was agreed in discussion on 9 Nov 2026
  but not yet confirmed as a decision.
- Why it matters: it fixes the shape of `Mode` in `key.rs`, today only
  `Major`. With the direction, `Mode` lists the seven modes (Aeolian is
  minor). The alternative, a fixed pair of major and minor, is simpler
  now but would need restructuring if modes come later.
- Options: confirm as written; or change it.
- Raised: 9 Nov 2026, this document, item 4.

**OPEN_QUESTION:** How is the song-level modes switch written?
- Context: a song declares once whether mode markings are annotations
  (off) or declarations that move the numerals' starting point (on),
  like `chord_numeral_standard` in item 1.
- Why it matters: it is a keyword in every song that uses modes.
- Options: to be proposed, in the style of `chord_numeral_standard`
  (a keyword and two values).
- Raised: 9 Nov 2026, this document, item 4.

**OPEN_QUESTION:** How is a mode marking written in a section?
- Context: a mode marking is set per section and changes only at a
  section boundary, like `key`. With the switch off it is shown to
  people only; with it on, numerals count from the mode's tonic.
- Why it matters: it decides what a section line looks like, and
  whether the key and the mode are one statement or two.
- Options: one statement (`key D dorian`); or two, the key and a mode
  beside it (`key C major  mode D dorian`).
- Raised: 9 Nov 2026, this document, item 4.

**5. The libraries.** *Requirements (Jim, 9 Nov 2026):* the harmony
(progression) libraries must support:

- both standards, `key_dependent` and `major_referential`, each entry
  stating its own;
- chords named by letter (`Am`, `E♭maj7`) as well as Roman numerals;
- slash chords that give the bass note (`Am/G`: A minor over G), which
  are not the song line's `/` (the same again) or slash rhythms.

**OPEN_QUESTION:** How is a chord with a given bass note written, so it
cannot be read as a classical secondary chord?
- Context: in chord symbols the slash gives the bass: `Am/G` is A minor
  over G. In classical Roman numerals the slash means a secondary chord:
  `V/V` is the V of V (D major in C major). Roman numerals give the bass
  by inversion figures (`V6`, `V64`) or, in pop usage, as `V/2` or
  `I/3`. The song line's `/` (the same again) is a third, separate use.
- Why it matters: the same slash would play different notes.
- Options: slash for the bass with letter chords only, and inversion
  figures for Roman numerals (keeping `V/V` classical); or slash for the
  bass everywhere, with secondary chords written another way.
- Raised: 9 Nov 2026, this document, item 5.

**OPEN_QUESTION:** How does a library entry state its chord numeral
standard?
- Context: each entry may use either standard (item 1), and a library
  holds many entries.
- Why it matters: an entry read with the wrong standard plays wrong
  chords with no error (item 1, "Where the two conflict").
- Options: the same `chord_numeral_standard` line in each entry; or one
  line per library file, with entries of the other standard in another
  file.
- Raised: 9 Nov 2026, this document, item 5.

## Where the code changes, once decided

`key.rs` (`Mode`), `contents_text.rs` (`parse_key`), `voicing.rs` (degree
roots in minor), `note_spelling` (chord-note spelling, flats or sharps),
`lilypond_export` (`\key … \minor`), the key wheel in `render`, and the
progression library format.

## Revisions

- 8 Nov 2026, revision 1: decision points framed.
- 9 Nov 2026, revision 2: answer to Jim's comment on BWV 610's key; "form" of the minor replaced by "scale" throughout, for precision.
- 9 Nov 2026, revision 3: decision point 1 rewritten: the two practices separated, each with where it is used and examples in C minor, a table of the same chords both ways, and where they conflict.
- 9 Nov 2026, revision 4: reference tables of the three minor scales and the major scale on the same tonic (steps 1 to 8, interval names, distances from the previous step and from the tonic) replace the background table; both practices restated in intervals above the tonic.
- 9 Nov 2026, revision 5: decision point 1's prose replaced by a table comparing the two practices, with columns for what each means for the program.
- 9 Nov 2026, revision 6: decision point 1 opens with the issue (how the program finds a chord's root in minor) and explains quality and voicing; Song Notation's Q12 quoted in point 4 and referenced in point 1.
- 9 Nov 2026, revision 7: decisions recorded: practice declared once per song (1), nothing to settle for signatures (2), key wheel dropped (3).
- 9 Nov 2026, revision 8: point 4: proposed direction recorded (common practice by tonal centre; a song-level switch making mode markings annotations or declarations; marked per section like a key change), still open.
- 9 Nov 2026, revision 9: point 4: background on modes (the seven modes, parent, key as tonic plus mode, signatures, numerals restarting at the mode's tonic, both practices in a mode, minor and modes).
- 9 Nov 2026, revision 10: decision 1 written as `chord_numeral_standard key_dependent | major_referential`; point 5: library requirements (both standards, letter chord names, slash chords with a bass note) and the open slash-meaning question.
- 9 Nov 2026, revision 11: every open question labelled OPEN_QUESTION with its context (five, in items 4 and 5); stale open remarks in items 1, 2 and 4 removed.
- 9 Nov 2026, revision 12: the two mentions of Song Notation's Q12 (items 1 and 4) no longer say "open question" loosely; item 4 points to OPEN_QUESTION Q12 there.
- 9 Nov 2026, revision 13: cross-references to Song Notation's Q12 removed from items 1 and 4; item 4 opens with what it is about.
