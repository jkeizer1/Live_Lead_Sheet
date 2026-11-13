# Live Lead Sheet — Song Notation

Revision 21 — 11 Nov 2026 (Jim's decisions on Q29 to Q33 written into sections 3 to 7; crescendo and diminuendo decisions and a list of items to confirm added to section 8; section 9 holds only what is still open)

**Status: draft for trial.** *Decided* marks what Jim has decided;
everything else is a proposal to try with a working player. Every open
question is labelled OPEN_QUESTION (search for it); the full list, with
context, is in section 9. Terms (bar, beat, pulse, tick, score position) are
those of `Musical Time and Clock.md`. In this document a rhythm's
**element** is one note or rest; a **sounding element** is a note.

## 1. The model in brief

The plain-text notation for songs, studio configurations and the two
libraries (EFS-6.1 to 6.5), kept in an adapter at the edge (A-3). It is
meant for hand editing, and for testing the player without the screen:
the full capability from text first, the user interface on top.

There are four kinds of file, each with its own extension.

| File | Extension | What it holds | Written by |
|---|---|---|---|
| Song (the composition) | `.song` | Tracks, sections, and for each track (or each voice layer of a track), bar by bar, the notes or chords it plays and their lengths. Cues and tempo marks. Nothing about ports, channels or devices. | Apply, then the musician by hand |
| Studio configuration | `.studio` | Which output ports exist, and for each track of a song: port, channel, octave offset, velocity levels, device timing. | The musician, once per studio |
| Rhythm library | `.library` | Rhythm patterns, in groups. | Starter file, and the musician |
| Progression library | `.library` | Chord progressions in Roman numerals, in groups. | Starter file, and the musician |

A song needs a studio configuration to be played, and one song can be
played through several studios: the composition says *what*, the
configuration says *where*. The two are matched by track name (section 6).

The editor action **apply** puts a progression and a rhythm onto the bars
of a section: the chords land on the rhythm's elements, and a bass line
can start from the chords' roots. Its result is written into the song.

*Decided by Jim:*

- **A song is self-contained** (4 Nov 2026). It holds copies of what it
  uses and refers to no library entry. The libraries are catalogues that
  will be edited and expanded; a song, once composed, stands alone.
- **Voice tracks play what is written** (5 Nov 2026). Nothing is looked up
  when the song plays. A chord element carries a chord name, voiced
  plainly unless changed; a note element carries its own pitch, which may
  be any note. Filling in roots is a convenience of the editor.
- **A note sounds for its written duration** (5 Nov 2026), with no
  gate-length percentage. Two marks change this per note: staccato
  shortens it and legato joins it to the next (section 4). A note that
  touches the next one on its track is shortened by 5 ms at the output, so
  the receiver sees a note off before the next note on (Q11).
- **Composition and studio configuration are separate** (5 Nov 2026), as
  above.
- **A song's pitches are staff pitches** (5 Nov 2026), written in the
  track's clef, so most notes sit on the staff and not on ledger lines.
  The device's own octave placement is the studio configuration's job.
- **Marks are annotated on the score** (5 Nov 2026): accents, staccato,
  legato and fermata, on any sounding element. How hard an accent plays
  is set per device in the studio configuration, not in the song.
- **Cues and tempo marks are written in rhythm notation** (5 Nov 2026),
  so the author never counts bars including repeats.
- **Polymeter is deferred** (5 Nov 2026). In V1 every line of a section
  has the same number of bars (section 5).
- **What an element plays comes first, then its length** (8 Nov 2026):
  `Eb4:sx`, `I:q`, `r:h`. A length left off is the one before it in the
  line, and `/` plays the chord or note before it again (section 4). The
  rhythm library keeps the note value first, since a rhythm plays nothing.
- **Voice layers** (8 Nov 2026). A voice track may hold several
  independent lines in a section, its voice layers, numbered 1, 2, 3 and
  so on, each filling its bars by itself. They share the track's port,
  channel and timbre and are drawn on its staff; a layer may be given a
  name, and is otherwise called by its number (section 5). This is how two
  voices on one staff are written (the soprano and alto of an organ's
  right hand).
- **Rests are written in full; engraving stays out of the song**
  (8 Nov 2026). Each line and each voice layer writes all its rests, even
  where printed music leaves them out to reduce clutter: the rest is what
  places the next note. Hidden rests, stem directions and other
  engraving choices are not in the song file; different engravers may
  draw the same song differently later.
- **Coupled and uncoupled lines** (11 Nov 2026). In a modular
  synthesizer, pitch, rhythm and loudness are separate streams; the
  "note" that couples them comes from standard notation. A track's line
  may be written coupled, as before (`I:q`), or uncoupled: a
  `sample_source` line holds the chords or notes, and a rhythm line
  beneath it holds only lengths and takes what it plays from the source
  (section 5, "Sample sources and uncoupled lines"). Standard music must
  work first; the uncoupled lines are what make this a modular sequencer
  and not one more notation program.
- **Phrase dynamics on their own line** (11 Nov 2026): `pp`, `p`, `mf`,
  `f`, `ff` and the crescendo and diminuendo marks, written in rhythm
  notation and taken up by each sounding element (section 5, "Dynamics,
  in rhythm notation"). Marks on single notes stay in the note line.
- **Every source line has its own play head** (11 Nov 2026). At first
  all play heads move together at the track's tempo. Separate play
  heads, so a source can run at its own tempo, are part of this
  program's first version and come before the GUI (section 8).

## 2. Rules shared by all files

- **Plain text, line by line.** Indentation is for the reader. A line
  whose first character, after indentation, is `#` is a comment, and the
  whole line is left out. After content, `---` starts a comment to the end
  of the line (`| r |  --- fires on each pass`), except inside a quoted
  name. Any other `#` is a sharp (`F#3`, `C#4:q`, `#iv°`, `key F# major`),
  never a comment; a sharp in the wrong place is reported with where a
  sharp goes. *Decided (Jim, 8 Nov 2026).*
- **Blocks.** A file holds blocks opened by a keyword and a quoted name:
  `rhythm "Charleston"`, `progression "12-bar blues"`, `song "Demo"`,
  `studio "Jim's studio"`. A library file holds only `rhythm` blocks or
  only `progression` blocks.
- **Bars** are written between `|`, as in a chord chart.
- **Units stay in the file** where a number is a quantity: `tempo 120
  beats/min`, `settle_delay 2 ms`, `octave_offset -1 octaves`. *Decided.*
- **Pitches** are written as a letter, an accidental (`#` or `b`) and an
  octave, with C4 the middle C of the piano: `C2`, `F#2`, `Bb1`. They are
  positions on the staff of the track's clef, not MIDI note numbers.
- **Chord names** use `#` and `b` for sharps and flats. The symbols `°` and
  `ø` are accepted, and so are ASCII spellings: `dim` and `dim7` for `°`
  (`viidim`), `m7b5` for `ø` (`iim7b5`). *Decided.*
- **Mistakes** are reported as in EFS-6.3: the file, the line, what is
  wrong and what to type instead. A file with a mistake does not load,
  not even partly, but **the program still starts** (*decided*, from the
  MIDI router's practice). It shows the errors, plays nothing from the bad
  file, and checks the file again each time it is saved. The editor
  includes a plain text pane, so the file can be fixed, saved and
  reloaded without leaving the program. If a song that is playing is
  edited into error, the last good version keeps playing while the errors
  are shown. This is Strudel's convention: a playing song is changed only
  by a valid edit; the editor keeps allowing edits, and a correct edit is
  then applied to the playing song. *Decided (Jim, 11 Nov 2026).*
  Example:

  ```
  14:02:11 WARNING: demo.song is not valid; nothing was applied.
           Issue 1: demo.song line 14: "IV9b" is not a chord. Try IV9 or IV7b9.
           NOT PLAYING: fix demo.song and save it; the program checks it each time.
  ```

## 3. Rhythm notation

A rhythm is one to eight whole bars of one time signature, written as
note values and rests. This section spells them as the rhythm library
does, note value first (`q. e hr`); a song's lines write the same values
after what each element plays (section 4).

| Letter | Value |
|---|---|
| `w` | whole |
| `h` | half |
| `q` | quarter |
| `e` | eighth |
| `sx` | sixteenth |
| `tx` | thirty-second |

A dot after the letter lengthens the note by half (`q.` is a dotted
quarter). A second dot adds half of what the first dot added, and a third
half of the second (`q..` is 1¾ quarters, `q...` 1⅞ quarters), as in
standard notation. An `r` makes a rest (`qr`); the `r` and the dots follow
the letter in either order, so `hr.` and `h.r` are the same dotted half rest.

```
rhythm "Quarter pulses"
  meter 4/4
  | q q q q |

rhythm "Charleston"
  meter 4/4
  | q. e hr |

rhythm "Rest, half, quarter"
  meter 4/4
  | qr h q |

rhythm "Quarter triplets"
  meter 4/4
  | trp (q q q) trp (q q q) |
```

A bar must add up exactly. Charleston is 1½ + ½ + 2 = 4 quarters, a full
4/4 bar.

**Groups.**

- `trp (q q q)` is three in the time of two: three quarters played in the
  time of two quarters, a half note. `trp (sx sx sx)` is three sixteenths
  in the time of two, an eighth.
- `fitQ(...)` fits what is inside into the length of one quarter note,
  dividing it in the proportions of the written values. `fitQ(e e e e e)`
  puts five equal eighths into one quarter. `fitE`, `fitH` and `fitW` do
  the same for an eighth, a half and a whole, and `fitBar(...)` for the
  whole bar. A triplet is the case `fitH(q q q)`, so `trp` is a short
  spelling of it. Every other tuplet is a `fit`.
- A `fit` over a bar is how an element lands off the beat: seven hits
  spread evenly over a bar fall on no quarter or eighth. The distribution
  is fixed by the notation. Random looseness would be generation (A-9) and
  belongs to a later transform.
- **Groups are functions, worked out into the score.** `trp` and `fit` are
  the first of a family: a function takes a set of elements and places
  them. The idea is borrowed from Strudel, which applies such functions
  live. Functions are allowed in `.song` files (*decided, Jim,
  11 Nov 2026*): a function stays in the file as it was typed, it is
  worked out each time the file is loaded, and the player plays the
  result. More functions can be added later, as features, without
  changing the player.
- **Euclidean rhythm.** `euclid(3,8)` spreads 3 hits as evenly as
  possible over 8 steps; the two parameters are the hits and the steps.
  The steps fill the bar, as `fitBar(...)` does: in a 4/4 bar the 8 steps
  are eighth notes, and in a 3/4 bar the same 8 steps are fitted evenly
  into the bar. A hit is a sounding element one step long, and the other
  steps are rests. *Decided (Jim, 11 Nov 2026); not yet built.*
  `euclid` only generates rhythm elements, so it is written where a
  length is written. In a rhythm library entry it stands alone:
  `| euclid(3,8) |`. In a song line it follows the colon:
  `C2:euclid(3,8)` plays C2 on every hit (a coupled line), and
  `:euclid(3,8)` takes what each hit plays from the track's
  `sample_source` line (an uncoupled line).

**Ties.** `~` joins an element to the one after it into a single note of
the combined duration. It may stand before a barline (here in the rhythm
library's spelling):

```
rhythm "Anticipation"
  meter 4/4
  | hr qr er e~ | q hr. |
```

The eighth on the "and" of beat 4 is held across the barline into
beat 1 of the next bar: one note an eighth plus a quarter long, a dotted
quarter. The rhythm line draws the tie across the barline (EFS-1.5). A tie
belongs to the rhythm, so every voice track can use it.

- **`~` is the tie mark.** Strudel uses `~` for a rest; here a rest is
  `r`. *Decided (Jim, 11 Nov 2026).*
- **A tie may not lead out of the last bar** of the bar range a rhythm is
  assigned to, of a section, or of a repeated group of bars. The loader
  refuses the song with a message. Jim accepts this limitation so that
  development can proceed; it is looked at again only if it proves to be
  a problem. *Decided (Jim, 11 Nov 2026).*

**Marks.** Written after the length (in the rhythm library, after the
letter, the dots and `r`), and before a tie.
They apply to a sounding element (a fermata may also stand on a rest). Each
mark changes only the element it is written after. The spellings are
decided (Jim, 11 Nov 2026). `>>` is called marcato, the term of standard
rudiments ("strong" is not a musical term). `<` and `<<` are the same
kind of mark in the other direction: the note is sent at a lower
velocity, and that is all they do. They have no name of their own and no
sign on the staff, since print has no articulation sign for one soft
note. *Decided (Jim, 11 Nov 2026).*

| Mark | Meaning | In the MIDI | On the staff |
|---|---|---|---|
| `>` | accent | velocity above the track's normal velocity, or the track's `accent` velocity if the studio gives one | accent sign |
| `>>` | marcato | velocity further above normal, or the track's `marcato` velocity if the studio gives one | marcato sign |
| `<` | a softer note | velocity below the track's normal velocity | nothing |
| `<<` | a still softer note | velocity further below normal | nothing |
| `'` | staccato | sounds for half its written duration | dot |
| `_` | legato: joined to the next note | the next note on is sent before this note's note off, so the two overlap; a mono receiver set to legato keeps its gate high | slur |
| `^` | fermata | nothing in V1 | fermata sign |

**How legato works in MIDI** (settled 11 Nov 2026). MIDI 1.0 has no
message that changes the pitch of a sounding note; a note's pitch is
fixed by its note on. Legato is therefore sent as a keyboard sends it
when a second key is pressed while the first is held: the second note on
first, then the first note off. Whether that makes a new gate is the
receiving device's setting. A mono receiver set to legato keeps its gate
high and only moves the pitch; set to retrigger, it drops the gate
briefly. If a device cannot do legato from overlapping notes, that is
the device's limitation. This is the opposite order to the 5 ms lead the
player gives a note off before a touching note on.

For a part a human plays (section 5, `musician_voice`) the marks are the
usual interpretation marks, shown and not sent. A fermata never changes
timing in V1: a leader sending MIDI clock cannot hold time without
disturbing every follower. The accent levels belong to the device (section
6); the rhythm only says where the accents fall, so a backbeat on beats 2
and 4 is part of a rhythm pattern, e.g. `| q q> q q> |`.

**Assigned to a bar range** (EFS-5.4): more bars than the pattern has, it
repeats from its first bar; fewer, it is cut after the last bar that
fits. A rhythm is refused for a section of a different meter, with a
message naming both.

**Not in a rhythm:** pitches and velocities. A rhythm's accent marks are
part of it: where the accent falls (beat 1 in 4/4; beats 1 and 2 in 6/8)
is written with `>`, so no meter has a rule of its own.

Example message: `charleston.rhythm line 3: bar 1 is short by an eighth
note. Add e or er.`

*How it maps to the clock.* Every element's place is a whole number of
ticks (1 tick = 1/5040 of a quarter note, D-7). The place of an element
inside a group is worked out from the start of the group, never by adding
rounded steps, so nothing drifts. A place that is not a whole number of
ticks is rounded to the nearest tick (*decided*). The error is at most
half a tick: at 120 q/min a quarter lasts 0.5 s/q, so one tick lasts 0.5
s/q ÷ 5040 ticks/q ≈ 0.099 ms/tick and half a tick is about 0.05 ms.
Besides groups such as eleven elements over a bar, a triple-dotted
sixteenth and a double-dotted thirty-second also fall on half ticks.

## 4. Elements, contents and marks in a line

A song line writes, for each element, **what it plays first, then its
length**, then its marks, then a tie: `Eb4:sx`, `C5:e.>`, `I:q`, `r:h`,
`D4:e~`. This is the order musicians name a note in ("E-flat, a
sixteenth"), and the order LilyPond and ABC notation write it in.

**What an element plays:**

- **Poly voice** (a track that can play chords): a chord name in the
  section's key (`ii7` in C major is D minor seventh), voiced plainly.
  The plain voicing (EFS-5.5) is in root position, meaning the root is
  the lowest note, and in close position, meaning the other notes are
  stacked directly above the root within one octave. The root is placed
  by the track's clef (section 5, "Tracks and their types"). Or notes:
  `[D3 F3 A3 C4]`. A chord name followed by notes (`ii7[F3 A3 C4]`) is a
  chord with its own voicing, for example without the root. A chord name
  followed by an inversion, `ii7[1stInversion]`, `ii7[2ndInversion]` and
  so on, is that inversion as standard music uses the term: the third is
  the lowest note in the first inversion, the fifth in the second.
  *Decided (Jim, 11 Nov 2026); inversions are not yet built.* How the
  other notes sit above the lowest one is proposed: the notes that were
  below it move up one octave.
- **Mono voice** (a track that can play one note at a time): one note with
  its octave (`C2`, `F#2`, `Bb1`). Two sounding elements may not overlap.
- **Cue control track:** a cue's name in quotes (`"Start SQ-1"`).
- **A rest:** `r`.
- **Nothing written:** `:q`, a length with nothing before the colon. This
  is an element of an uncoupled rhythm line; it takes what it plays from
  the track's `sample_source` line (section 5, "Sample sources and
  uncoupled lines"). The colon is always written here.
- **The same again:** `/` plays what the sounding element before it in
  the line played (a strum is several elements of one chord); rests in
  between do not count. A `/` takes its own length and marks: `/`, `/:e`,
  `/>`. A cue control track names every cue; `/` is refused there.
- A beat pulse needs no section of its own: it is a mono voice that writes
  the same note on each element, e.g. `| C4:q> / / / |`.

**Lengths.** The note values and dots of section 3: `w h q e sx tx`, and up
to three dots. **A length left off is the one before it** in the line,
dots included, across barlines and across the text lines a line goes on
over. The first element of a line writes its length. Marks and ties never
carry; each belongs to one element.

**The colon** separates what an element plays from its length. It may be
left off where nothing is ambiguous: after a pitch, whose octave ends it
(`Eb4sx`, `C5e.>`), after notes in brackets or a quoted cue name, and after
`r` and `/`. **After a chord name the colon is required** (`I:q`,
`ii7:h.`), because a chord name has no fixed end: `Iq` is refused with the
advice `write I:q`. The colon may always be written, even with nothing
after it (`D4:~`). Marks and ties need no colon, chords included (`ii7>`,
`I~`), since none of `> ' _ ^ ~` appears in a note or chord name.

**Groups.** Inside `trp` and `fit` an element takes and carries its written
length as anywhere else, and the group squeezes it. After the group, a
length left off is the written one, not the squeezed one: in
`| trp (C4:q D4 E4) F4 G4 |` the F4 and G4 are plain quarters.

**Order inside an element** (fixed by the parser): what it plays, then `:`
and the length with its dots, then the marks, then `~` (`C2:q>>`,
`ii7:e'~`, `D4~`). An element ends at the next space, except inside `[...]`
or a quoted cue name.

**Examples**, in the order of Revision 14 and in this one:

| | Revision 14 | Revision 15 |
|---|---|---|
| A melodic line (BWV 610, alto, bar 1) | `sx:Eb4 sx:D4 e~:Eb4 sx:Eb4 sx:F4 sx:Eb4 sx:D4 e:C4 e~:D4 e:D4 e:C4` | `Eb4:sx D4 Eb4:e~ Eb4:sx F4 Eb4 D4 C4:e D4~ D4 C4` |
| Every length changes (BWV 610, soprano, bar 5) | `e.:C5 sx:D5 q:Eb5 q.:D5 e:D5` | `C5:e. D5:sx Eb5:q D5:q. D5:e` |
| Rests (BWV 610, pedal, bar 1, first half) | `er sx:C3 sx:D3 sx:Eb3 sx:D3 e~:Eb3` | `r:e C3:sx D3 Eb3 D3 Eb3:e~` |
| Chords, the same again | `q:I q q q` then `qr h:ii7 q:V7` | `I:q / / /` then `r ii7:h V7:q` |
| A beat pulse | `q>:C4 q q q` then `q> q q q` | `C4:q> / / /` then `/> / / /` |
| A bass with rests | `h:C2 qr q:G2` then `q:G2 q:B2 q:D2 q:G2` | `C2:h r:q G2` then `G2 B2 D2 G2` |
| A mark with no length (3/8) | `sx:C6 sxr sx>:C5 sxr sx:C4 sx:C5` | `C6:sx r C5> r C4 C5` |
| A cue | `w:"Start SQ-1"` then `wr` | `"Start SQ-1":w` then `r` |
| A group | `trp (q:C4 q:D4 q:E4) h:F4` | `trp (C4:q D4 E4) F4:h` |

In the first example `D4` is a sixteenth, carried from `Eb4:sx`; `Eb4:e~`
is a tied eighth, and `Eb4:sx`, where the tie lands, is a sixteenth again.
The Debussy case, a half note on beat 3 of an inner voice printed with no
half rest before it, is written with the rest: `| r:h A4~ | A4 … |`
(section 1).

A line written in the order of Revision 14 is refused, with a message
giving the new spelling:

```
line "Keys": "q:I": a song line writes what it plays first, then its length, e.g. C4:q, I:q or r:q, not q:C4
line "Bass": "C2" has no length, and nothing before it in this line gives one. Write it, e.g. C2:q
```

## 5. The song (`.song`)

A song names its tracks and its sections. A section holds, for each track,
a *line*: the track's rhythm, bar by bar, with the notes each sounding
element plays. Apply writes these lines. Nothing in a song names a port,
a channel or a device.

```
song "Demo"

track "Clock"
  type clock_and_transport

track "Beat"
  type mono_voice
  clef treble

track "Keys"
  type poly_voice
  clef grand
  lead

track "Bass"
  type mono_voice
  clef bass

track "Cues"
  type cue_control
  cue "Start SQ-1"  destination "SQ-1"  start

section "Count-in"  count-in
  meter 4/4  tempo 120 beats/min
  line "Beat"  | C4:q> / / / |

section "Verse"  repeat 2
  key C major  meter 4/4  tempo 120 beats/min
  line "Beat"  | C4:q> / / / | /> / / / |
  line "Keys"  | I:q / / / | r ii7:h V7:q |
  line "Bass"  | C2:q / / / | r D2:h G2:q |
  line "Cues"  | "Start SQ-1":w | r |   --- fires on each pass: passes are not designed (EFS-4.4)
```

### Tracks and their types

A track has a name (unique in the song; lines and the studio
configuration refer to it), a `type`, and for voices a `clef`. The order
the tracks are written is the order they are shown. The name states the
intended voice ("Keys", "Bass"); the type states what the track can do,
and so what a receiving device must be able to do.

| `type` | What it is | In V1 |
|---|---|---|
| `poly_voice` | Notes and chords to a device that can play several notes at once. | Yes |
| `mono_voice` | One note at a time, to a mono device (an analogue oscillator, a bass synth). The loader refuses overlapping notes, in hand-written files as well as the editor's. | Yes |
| `clock_and_transport` | Timing Clock from Play to Stop, and Stop. Exactly one per song. Its output has a port but no channel, because Timing Clock, Start, Continue, Stop and Song Position Pointer carry no channel. | Yes |
| `cue_control` | Phrase cues, placed in rhythm notation (below). | Yes |
| `tempo_control` | Stepped and ramped tempo changes inside a section, in rhythm notation (below). | Later |
| `musician_voice` | A part composed for a person to read and play, in the clef of that instrument. Sends no MIDI; drawn on its own staff, with marks as interpretation. | Later. It is written in the `.song` file and rendered; in the visual player it is a first-class track that can be focused and shown with a play head. The display details are settled with the UI (*decided, Jim, 11 Nov 2026*). |
| `automation_control` | Continuous curves sent as controllers (a filter sweep, a crescendo on a VCA). | Later |

**Every voice is the same on the wire:** a note-on with a velocity at the
start of an element and a note-off at its end. The written length is the
note's length, which at a CV and gate output is the gate length (staccato
halves it), so a short element is a trigger and a long one a held gate. How
the receiver turns the note into signals is the receiver's setting, and
neither file records it. An FH-2 output can be a velocity gate, a CV and
gate, or a CV, gate and velocity; the Disting NT can take the same MIDI
directly, from the FH-2's forwarding or from its own port; in a velocity
gate the velocity is the gate's level, so the accent levels in the studio
file matter most there. The `retrigger_gap` of the studio file drops the
gate between two notes.

**A beat pulse is a mono voice** that writes one fixed pitch on each
element, with `>` where the accent falls: `| C4:q> / / / |`. Its pitch is a
pitch like any other (with `octave_offset` to suit the device, and the
pitch CV it makes at a CV and gate output), and it needs no transport. It
can clock a divider, or voice a metronome in the modular.

Attributes of a voice track:

- `clef` — `treble`, `bass`, `alto`, `tenor`, `percussion`, or `grand`
  (treble and bass braced, for chords that run well below middle C). The
  composer composes in the clef; the music stays on the staff and not on
  ledger lines. The rhythm line always uses the percussion clef (EFS-1.5).
- `lead` — exactly one voice track carries it: it is the song's
  harmony track, and the chord symbols show its chords. When the lead track has several
  voice layers, the chords are read from voice layer 1 (proposed; nothing
  reads chord symbols yet).
- `voice_layer n "Name"` — optional, one line per name: a name for one of
  the track's voice layers, e.g. `voice_layer 2 "Inner"`. A layer without
  a name is called by its number, and the editor shows the number until a
  name is given. Numbers and names may not repeat within a track.
- **Home octave.** The octave where a plain voicing puts the root and
  where apply puts a bass line's roots comes from the clef: treble 4,
  alto 3, tenor 3, bass 2, grand 4. There is no `home_octave` setting;
  it was removed on 11 Nov 2026. *Decided (Jim, 11 Nov 2026).*
- **Octave annotations.** `+8va` and `-8va` move what follows one octave
  up or down from the clef's octave. One may be written at the start of
  any bar, most often at the start of a section, on any track with notes
  or chords, and it stays in effect until it is changed. `loco`, the
  word print uses, returns to the clef's octave. The register of a
  device is a different matter, set by `octave_offset` in the studio
  configuration (section 6). *Decided (Jim, 11 Nov 2026); not yet built.*
- `instrument` — for `musician_voice`: a label, typed freely at the
  musician's discretion. The clef follows standard notation; Appendix C
  gives examples only. The track states its own clef and written
  transposition; that spelling is designed when `musician_voice` tracks
  are built. *Decided (Jim, 11 Nov 2026).*

**Pitch is stored as sounding pitch.** A `musician_voice` for a trumpet
is stored as the concert pitch that sounds, and drawn a major second
higher, as notation software does. Transposing instruments (Appendix C)
are the only place a written pitch differs from a sounding one.

### Voice layers

A voice track's line in a section may name its voice layer after the
track's name; a line that names none is voice layer 1. A section holds at
most one line per track and layer, and every layer fills its bars by
itself, with its rests written (section 1). The layers of one track share
its port, channel and timbre: they are independent rhythms on one
instrument, not separate instruments. A mono voice has only voice layer 1.

```
track "Manual"
  type poly_voice
  clef grand
  lead

section "Stollen"  repeat 2
  key C major  meter 4/4  tempo 40 beats/min
  line "Manual"  voice_layer 1  | G4:q G4 F4 Eb4 | …
  line "Manual"  voice_layer 2  | Eb4:sx D4 Eb4:e~ Eb4:sx F4 Eb4 D4 C4:e D4~ D4 C4 | …
  line "Manual"  voice_layer 3  | C4:sx B3 C4:e~ C4:sx B3 C4 G3 A3:e G3~ G3:sx G3 Ab3 Eb3 | …
```

(BWV 610 is in C minor; the example says C major only because minor keys
are not built yet. The whole piece is `songs/LilyPond/bwv610.song`.)

In the vocabulary a line has a **role** for its track: a voice layer with
its number, or the placing of a cue control track's cues. More roles, for
the decorating streams of `docs/Concept/concept_idea.md`, are to come; a
track can then hold several lines of different roles.

Two voice layers of one track may hold the same pitch at once (BWV 610
does this seven times between its chorale and inner voices). On one MIDI
channel a note is known only by its pitch, so the two cannot be told
apart, and a note off for one would end both. The player therefore sends
one note: a note on at the first start, no second note on when the other
voice layer strikes the pitch while it sounds (the effect is legato,
with no new gate), and the note off only when the longest of the notes
has finished, ties honoured. A short note against a long note of the
same pitch in another voice layer is a composer's error; the longest
note has priority. *Decided (Jim, 11 Nov 2026); not yet built.*

### Sections

`section "Name"` with `count-in` for the first section if wanted, `repeat
n`, and `key`, `meter` and `tempo` (or carrying on from the previous
section). A key change, a time-signature change and a stepped tempo change
all happen at a section boundary; a change that lasts a few bars is a
short section. The initial tempo is the first section's. The key
signature is drawn on the staff the first time, and again everywhere the
key changes, as standard notation requires (*decided, Jim, 11 Nov 2026*).

- **Section length** is the number of bars in its lines. In V1 all lines
  in a section, voice layers included, have the same number of bars, and a track with no line in a
  section rests. That is how the voices rest in the count-in section
  (D-10). *Polymeter* (a line that is a shorter cycle repeated to fill
  the section) is deferred; if it is added, such a line must be marked as a
  cycle, so a line left short by mistake stays an error.
- **Repeats** are written out in play order by the loader (EFS-1.6). A
  section with no `repeat` plays once. The number is the total number of
  times the section plays: `repeat 2` plays it twice, `repeat 3` three
  times.
- **Repeat signs inside a section.** `|%` opens and `%|` closes a group
  of bars that is played again, as the repeat signs of standard notation
  do: `|% I:w | V %|` plays the two bars twice. A number after the
  opening sign is the total number of times the bars play, counted as
  for `repeat n`: `|%2` is twice and `|%3` three times; with no number
  the bars play twice. *Decided (Jim, 11 Nov 2026); not yet built.* There
  is no sign for "the previous bar again": `/` already plays the previous
  note or chord again. A sign `..` was considered and dropped.
- **Carrying on.** A section that omits `key`, `meter` or `tempo` carries
  on the one before. The first section must give `meter` and `tempo`
  (nothing is assumed). A section with chords needs a key, written or
  carried; a count-in section needs none. `count-in` is for the first
  section only, one or two bars.
- **Tempo counts beats**, as the meter defines them (`Musical Time and
  Clock.md`, sections 2 and 13): `tempo 120 beats/min` or `tempo 2
  beats/s`. The number may be fractional (`tempo 112.5 beats/min`) and is
  held exactly, with no rounding (*decided, Jim, 11 Nov 2026*). No note
  value is written. In simple time a
  beat is the lower figure's note; in compound time (6, 9 or 12 on top) it
  is three of them, a dotted note. So `meter 3/8  tempo 30 beats/min` is
  30 eighths a minute, and `meter 6/8  tempo 60 beats/min` is 60 dotted
  quarters a minute: 60 beats/min × 3 eighths/beat = 180 eighths a minute.
  `q/min`, `e/min` and the like are refused, with a message saying what to
  write. *Decided (Jim, 7 Nov 2026).*
  - **A carried tempo keeps the note speed.** A section that changes the
    meter but writes no tempo keeps the speed of the notes, not the number
    of beats: after `meter 4/4  tempo 120 beats/min`, a section with only
    `meter 6/8` plays quarters at 120 a minute, which is 120 q/min ÷
    1.5 q/beat = 80 beats/min. *Decided (Jim, 7 Nov 2026).*
  - **The same number with a new beat is a new speed.** After 4/4 at
    `tempo 120 beats/min`, a 6/8 section writing `tempo 120 beats/min`
    plays 120 dotted quarters/min × 1.5 q/dotted quarter = 180 q/min: the
    quarter notes speed up from 120 to 180 a minute.
- **Meters** are n/d with d = 1, 2, 4, 8 or 16. Compound meters stop at
  12 on top: 15/8, 18/8 and other multiples of 3 above 12 are refused when
  the song is read, with a message, until their beats are decided
  (`Musical Time and Clock.md`, section 2). Other figures above 12 (13/8)
  are simple time.
- **Beat grouping.** A meter such as 7/8 or 5/4 may state how its bar is
  grouped: `meter 7/8  grouping 2+2+3`, `meter 5/4  grouping 3+2`. The
  grouping sets where the accents fall and how the notes are beamed. The
  beat stays the lower figure's note (the eighth note in 7/8), so the
  tempo and the counter are unchanged. With no grouping written, the beat
  and the pulse are identical. *Decided (Jim, 11 Nov 2026); not yet
  built.*
- **Order in the file.** All `track` blocks come before the first
  `section`; a `line` names a track already declared. Settings (`key`,
  `meter`, `tempo`, `repeat`, `count-in`) may stand on the section line or
  on lines after it, in any order.
- **A long line goes on over several text lines.** A text line that starts
  with `|` adds its bars to the `line` directly above it, as bars do in a
  rhythm (section 3). Comment lines may stand between; a setting ends the
  line, so bars after a setting, or before any `line`, are refused with a
  message. The track's name may stand alone, with all its bars below:

  ```
  line "Keys"  | I:h / | V / | vi / | IV:w |
  # second phrase
               | I:h / | V / | I:w  | /    |
  line "Bass"
    | C2:q / / / | G2 / / / | A2 / / / | F2 / / / |
    | C2 / / /   | G2 / / / | C2:w     | /        |
  ```

  Bars are numbered along the whole line, and a message names the text
  line the bar is on: `line 34: line "Keys": bar 7 is short by a quarter
  note`. *Decided (Jim, 7 Nov 2026).*

### Sample sources and uncoupled lines

*Decided (Jim, 11 Nov 2026); built, with one play head.*

What a track plays and when it plays it can be written on two lines of
the same track. Each track can be written coupled or uncoupled.

- **Coupled line.** Each element says what it plays and for how long
  (`I:q`, `C2:h`). This is the line described so far; it stays valid and
  needs no source.
- **Uncoupled lines.** A line marked `sample_source` holds chords or
  notes with no lengths. A plain line beneath it is the rhythm: its
  elements have a length and nothing before the colon (`:q`).

```
line "Keys"  sample_source  | I        | IV IV V     |
line "Keys"                 | :h :q :q | :q :q :q :q |
```

- **The source shares each bar evenly.** In 4/4, `I` fills the first bar,
  and `IV IV V` each last a third of the second bar, 1⅓ quarter notes.
  Four chords in a 4/4 bar last a quarter note each; two chords in a 3/4
  bar last a dotted quarter note each.
- **The rhythm is the gate.** Every rhythm element sounds. When it
  starts, it plays what the source holds at that moment, with a new
  gate. In the example the four quarter notes of the second bar play IV,
  IV, IV, V.
- **Track and hold.** If the source changes while an element is still
  sounding, the element changes to the new chord or note legato: the new
  notes are sent before the old ones are released, with no new gate and
  so no new envelope. With the rhythm `| :h :h |` over `| IV IV V |`, the
  half note on beat 3 starts on the second IV and moves to V two thirds
  of the way through beat 3. The resolving chord is heard. A chord is
  never skipped while the track is sounding, because losing a resolving
  chord would change how the progression is heard.
- **During a rest the gate is closed.** A change of the source that falls
  wholly inside a rest is not heard.
- **Legato depends on the receiver.** A mono receiver set to legato keeps
  its gate high. A poly receiver gives each new note its own voice and
  envelope, so a change between chords with no notes in common is heard
  with new attacks. That is the device's behaviour.
- **Marked, not guessed.** A line is a source only when it says
  `sample_source`. On any other line a note or chord without a length is
  still an error, so that typing mistake is still caught.
- **One play head at first.** The source and the rhythm move together,
  bar for bar, at the track's tempo, so "what the source holds at that
  moment" is the same bar and beat of the source. A source line later
  gets its own play head and may run at its own tempo, keeping the
  track's time signature.

In modular terms the rhythm is the gate and the source is the voltage
being tracked: a track and hold, whose input is the sample source.

### Dynamics, in rhythm notation

*Decided (Jim, 11 Nov 2026); not yet built.*

Loudness over a phrase is written on a line of its own, marked
`dynamics`. It is separate from the marks on single notes (section 3,
"Rhythm notation", under "Marks"), which still change only the note they
are written after: the backbeat `| q q> q q> |` is unchanged.

```
line "Keys"            | I:w | IV | V  | I  |
line "Keys"  dynamics  | p:w | <  | f  | mf |
```

This reads: quiet, grow through the second bar, loud at the third,
normal from the fourth.

| Mark | Meaning |
|---|---|
| `pp` | pianissimo |
| `p` | piano |
| `mf` | mezzo-forte, the normal level: velocity 64, or the track's `velocity normal` if the studio gives one |
| `f` | forte |
| `ff` | fortissimo |
| `<` | crescendo, growing through its element's length |
| `>` | diminuendo, falling through its element's length |

- **Written like the cue line.** Each mark is an element with a length,
  and `r` is a stretch with no change. Lengths carry forward as on any
  line. A crescendo over the second half of a bar is `| r:h <:h |`.
- **A level holds until the next mark,** as in print. There is no "back
  to normal" sign: the normal level is written, `mf`.
- **A song with no dynamics line plays at `mf` throughout.**
- **The levels are relative to `mf`.** `pp` and `p` lie below the
  track's normal velocity and `f` and `ff` above it, so all four shift
  when the studio gives the track another `velocity normal`. *Decided
  (Jim, 11 Nov 2026).*
- **Each sounding element takes its loudness from the dynamics line** at
  the moment it starts, as a rhythm takes its chord from a sample
  source. Inside a crescendo or diminuendo the level moves in a straight
  line in time, from the level before it to the level written after it
  (decisions in section 8, "Decided for the marks and the dynamics line").
- **`<` and `>` have two homes.** On a dynamics line they are the crescendo and the diminuendo.
  After a note in a note line they are the marks for a single note.
- **On the screen** the line is drawn above the staff, or its highest
  ledger line, and below the chords. The player shows the letters in
  italics and the crescendo and diminuendo as elongated symbols.
- **`mp`** (mezzo-piano) is left out until it is wanted.
- **One play head at first.** The dynamics line moves with the track.
  Like a sample source, it can later have its own play head and tempo,
  for example a crescendo cycle of a different length to the harmony
  beneath it.

### Cues, in rhythm notation

A `cue_control` track declares its cues by name: the message, and the
*destination* it goes to. The destination is a name, mapped to a port and
channel in the studio configuration (section 6), so the song stays free of
ports. The cue is then *placed* by a line in rhythm notation, where an
element that would carry a note carries a cue name:

```
track "Cues"
  type cue_control
  cue "Start SQ-1"    destination "SQ-1"  start
  cue "SQ-1 pattern"  destination "SQ-1"  program 3
  cue "Reset divider" destination "FH-2"  note 36 velocity 100
```

- A cue fires at the start of its element. Counting and repeats are the
  rhythm's, so no bar numbers are written: `| "Start SQ-1":w | r |`.
- For a `note` cue, the element's written duration is the note's length: a
  short note is a trigger, a held one a gate (EFS-4.3). For the other
  messages the duration does not matter.
- Messages: `note`, `cc`, `program`, and on this track type `start`,
  `stop`, `continue` (a destination for these needs a port only).
- A cue whose destination is `track "Keys"` goes to that voice track's own
  port and channel and is drawn above that track's staff (EFS-4.3, D-10).

### Tempo marks, in rhythm notation (later)

A `tempo_control` track is a line in rhythm notation whose sounding
elements carry a tempo mark:

- `step 100 beats/min` — the tempo becomes 100 beats/min at the start of
  the element.
- `ramp 60 beats/min` — the tempo moves from what it was at the start of
  the element to 60 beats/min by the end of the element. A ramp lasts the
  element's written duration; a tie makes it longer. A ritardando over the
  last four bars, in this revision's order: `| "ramp 60 beats/min":w~ | /~ | /~ | / |`.

The beats are those of the section's meter, as for `tempo`.

Ramps apply to any stretch of a section (a speeding up in the first bars,
a slowing mid-phrase, a retard at the end after several bars at tempo).
All devices follow, because they follow the clock. Ramps need a design in
`Musical Time and Clock.md` first: its instants are computed from anchors
with a constant tempo each, and a ramp is a continuously changing tempo.
Until then, tempo changes are stepped, per section. A gradually
increasing or decreasing tempo is to be smooth, without jumps (*decided,
Jim, 11 Nov 2026*).

### What the song does not say

Chord symbols read the chords carried by the lead track (the harmony
track), at the element where each chord starts. A bass line is free: it
plays the roots in the example, but nothing requires it.

**Not in the song at all:** engraving choices, such as rests left out
to reduce clutter, stem directions and where a voice layer crosses to the
other staff of a grand staff. The renderer decides them.

## 6. The studio configuration (`.studio`)

The configuration is the realisation of a song in one studio: which ports
exist, and what each track of the song is sent to. It is written once and
used for any song whose track names it covers.

```
studio "Jim's studio"

port "Clock"
port "Harmony"
port "Bass"
port "Beat pulse"
port "To SQ-1"

track "Clock"
  port "Clock"

track "Beat"
  port "Beat pulse"  channel 15
  velocity normal 64  accent 127

track "Keys"
  port "Harmony"  channel 1
  velocity normal 90  accent 105  marcato 120

track "Bass"
  port "Bass"  channel 2
  octave_offset -1 octaves
  velocity normal 100  accent 115
  retrigger_gap 2 ms

destination "SQ-1"
  port "To SQ-1"  channel 1
```

- **Ports** are declared first, one `port "Name"` line each, before the
  first `track` or `destination` block; inside a block `port` chooses
  which declared port the block uses. Ports are the program's named MIDI
  output ports (EFS-3.1). Exactly
  one is the clock output (EFS-2.5): the port of the `clock_and_transport`
  track. Connecting ports to devices is not here: that is the studio's job
  (a MIDI router, `aconnect`).
- **A track block** is matched to the song's track of the same name. It
  gives the port and, except for the clock track, the channel (1 to 16).
  Every track of the song that sends MIDI must have one, or the song does
  not load, with a message naming the track and the file. A cue control
  track has no block of its own: its cues go to destinations. A voice
  track's block may give `velocity normal` and a velocity for each mark
  its lines use (see `velocity` below). A block for a
  track the song lacks is ignored, so one configuration serves many songs.
- **`octave_offset`** shifts every note sent, in octaves, to where this
  device places the pitch (the manufacturers disagree where C0 is, and an
  oscillator may sit in a different register). Omitted, it is 0 octaves, a
  harmless default: the note is sent as written.
- **`velocity`** gives the MIDI velocity (1 to 127) for a normal note and
  for the marks the song uses. *Decided (Jim, 11 Nov 2026).* Built on
  11 Nov 2026: `normal` is 64 when not written, the word for `>>` is
  `marcato`, and `strong` is refused with a message saying to write
  `marcato`. The relative levels are built too (see below): a studio
  block no longer has to give an `accent` or `marcato` velocity.
  - `normal` is the track's normal level, which Jim called
    `volume_normal`. When it is not written it is 64, the value the MIDI
    1.0 specification gives for a mezzo-forte note.
  - The marks `<<`, `<`, `>` and `>>` are relative to the normal level,
    unless the block gives an absolute velocity for a mark (`accent` for
    `>`, `marcato` for `>>`). `<` and `<<` have no word here and are
    always relative (*decided, Jim, 11 Nov 2026*).
  - The levels `pp`, `p`, `f` and `ff` of a dynamics line are relative to
    the normal level too, and shift with it.
  - **The numbers are a share of the room left** (*decided, Jim,
    11 Nov 2026; built for the marks on a note; the levels of the
    `dynamics` line are not yet built*). A MIDI velocity runs from 1 to 127.
    `f` is 3/8 of the way from the normal velocity to 127, and `ff` is
    3/4 of the way. `p` is 3/8 of the way from the normal velocity to 1,
    and `pp` is 3/4 of the way. A mark on a single note does the same
    from the level in force at that note: 3/8 of the way for `<` and
    `>`, 3/4 for `<<` and `>>`. The result is rounded to the nearest
    whole velocity. Nothing reaches the limit, so `f` and `ff` stay
    apart on a track whose normal velocity is high.

    | Normal velocity | `pp` | `p` | `mf` | `f` | `ff` |
    |---|---|---|---|---|---|
    | 64 | 17 | 40 | 64 | 88 | 111 |
    | 90 | 23 | 57 | 90 | 104 | 118 |
    | 100 | 26 | 63 | 100 | 110 | 120 |

    Worked for a normal velocity of 100: the room above is 127 − 100 =
    27, so `f` is 100 + 3/8 × 27 = 110 and `ff` is 100 + 3/4 × 27 = 120;
    the room below is 100 − 1 = 99, so `p` is 100 − 3/8 × 99 = 63 and
    `pp` is 100 − 3/4 × 99 = 26. All of these are MIDI velocities.
  - **No loudness curve is applied.** The song and the player send
    velocities, and the studio configuration says nothing about how a
    device turns a velocity into loudness. That is the receiving
    device's business. *Decided (Jim, 11 Nov 2026).*
- **Device timing**, optional and omitted when not needed: `retrigger_gap`
  (a stopgap that raises the 5 ms lead of a note off before the next note
  on above its floor; Q11) and `settle_delay` (time for a quantizer's voltage to settle after
  a note is sent). Both are durations in ms, sent as clock time where the
  MIDI is sent, not in ticks: a device's response time does not change
  with tempo (A-7). Omitted, each is 0 ms, a harmless default: no change
  to the timing. More such device settings are expected as they are found.
- **Destinations** are the names cues use. A destination for Start, Stop
  and Continue needs only a port.
- Staccato sounds for half the written duration, fixed in V1; it can move
  into the configuration if listening shows a device needs another value.

## 7. The libraries (`.library`), and apply

The libraries hold what the editor applies, in plain text in the same
notation (EFS-6.5). There are two files, one for rhythms and one for
progressions. Each entry has a `group`, so the editor can offer a tree and
a file stays one list.

```
rhythm "Charleston"
  group "Swing"
  meter 4/4
  | q. e hr |

rhythm "Backbeat"
  group "Rock"
  meter 4/4
  | q q> q q> |

progression "12-bar blues"
  group "Blues" "Full form"
  mode major
  meter 4/4
  | I7 | I7 | I7 | I7 | IV7 | IV7 | I7 | I7 | V7 | IV7 | I7 | V7 |

progression "ii-V"
  group "Jazz" "Cadence"
  mode major
  meter 4/4
  | ii7 V7 |
```

- **Rhythms** are grouped first by meter (it is the entry's `meter`, so it
  is not written again), then by `group`, a genre or family: *Pulses*,
  *Swing*, *Rock*, *Latin*, and a compound-meter family for 6/8. The
  starter set is the one in EFS-7.2; the musician adds groups.
- **Progressions** have two `group` names: an idiom, then a kind. The
  starter set (EFS-7.1) is grouped as *Blues* (full form: 12-bar blues,
  quick-change, jazz blues, minor blues, 8-bar blues), *Jazz* (cadence:
  ii–V–I major and minor; turnaround: I–vi–ii–V), *Pop and rock* (loop:
  I–V–vi–IV). Classical material, such as the harmonic transitions Jim
  will take from composers like Fernando Sor, is added later as its own
  group.
- **A progression** is its chords bar by bar, in Roman numerals following
  the naming of EFS-1.5a (`I7`, `IM7`, `ii7`, `vii°`, `bVII7`, `V7b9`). It
  has no key, and it carries no rhythm or timing, only the sequence: the
  chords in each bar, over one or more bars. The chords of a bar share
  the bar evenly. *Decided (Jim, 11 Nov 2026).*
- **A rhythm** is as in section 3, with its marks.

**Apply** takes a progression, a rhythm, a bar range and a track, and
returns the track's line for those bars. In the editor it is a drag; in
the first tests it is a function run from the terminal. Its rules are
conveniences, and everything it writes can be changed by hand.

Apply writes a coupled line: each element carries its chord (`IV:e`),
as before there were sample sources. A coupled line holds its chords
itself and is never sampled from a source. Only an element with nothing
before the colon (`:e`) looks at the track's `sample_source` line, and
such uncoupled lines are written by hand, with nothing for apply to
place. *Decided (Jim, 11 Nov 2026).*

The rules:

1. The bar's first chord goes on the bar's first sounding element, and the
   later elements carry it on. For `| qr h q |` and the chord `ii7` the
   chord goes on the half note. The rhythm's marks are copied onto the
   elements.
2. Each later sounding element takes the chord in effect when it
   starts, the bar's chords sharing the bar evenly (*decided, Jim,
   11 Nov 2026*). A tie keeps the chord of the element it continues.
3. A bass line starts with the root of the chord in effect at each
   element, in the track's home octave. It may then be changed to any
   note, including the root in another octave.
4. A bar with more chords than sounding elements is reported, naming the
   bar and the chords. The extra chords are left off for the musician to
   place.
5. A harmony line can be written with each chord's root left out (the
   notes in brackets, section 4), for a studio where the bass plays the
   root.
6. The song keeps the result and no link to the library entries.

*Example.* The progression `ii-V` and the rhythm `Rest, half, quarter` in
C major give `| r:q ii7:h V7:q |` for the harmony track and `| r:q D2:h
G2:q |` for the bass. The same progression under `q q q q` gives `| ii7:q /
V7 / |`. How the editor snaps to a grid, and what it asks when rule 4
applies, are decided with the editor.

## 8. Build order, tests and where the code lives

The player comes first: a working player is how these ideas are tried.
Each test has an example under `examples/<what>_test/` and its
`explainer.md`.

1. `rhythm_notation_test`: loads the starter rhythms, prints each element
   in the musician's terms (for example "bar 1 beat 2: a half note"), checks the
   error messages, then plays a chosen rhythm through the beat-pulse port
   at a chosen tempo, for Jim to hear on the FH-2.
2. `song_notation_test` (built): loads `songs/Demo.song` and
   `songs/Demo.studio`, prints the song in the musician's terms, checks
   that the studio covers every track and cue destination the song uses,
   and checks the messages for nine kinds of mistake.
3. `song_play_test` (built, not yet heard): loads `Demo.song` and `Demo.studio` and plays them:
   count-in, beat pulse, harmony, bass and the SQ-1 Start cue. The player
   turns a song and a studio into note tracks and cue tracks
   (`player::song_plan`, with the chords voiced by `player::voicing`); it is
   the first time Jim hears a composed song. Accents are tried here;
   staccato and legato need the retrigger gap (Q11) and are tried after it.
4. `apply_test`: from the terminal, choose progressions and rhythms and
   bar ranges and repeats, apply them to a song structure, write the
   song, and hand it to `song_play_test`'s player. This is the test Jim
   described: select, apply, hear.
5. `retrigger_gap_test`, the spike of Q11, can run at any time.
6. The editor, with its text pane, once these work from text.

**Order of the work** (*decided, Jim, 11 Nov 2026*). Separate play heads
are part of this program's first version, not a later version.

1. Coupled `.song` files play from text. This is nearly there.
2. `sample_source` and `dynamics` lines at the track's tempo, tested from
   text files.
3. Separate play heads, so a source line can run at its own tempo,
   tested from text files.
4. The GUI, which from the outset deals with several play heads.

**Decided on 11 Nov 2026 and not yet in the code:**

- The `dynamics` line.
- Legato sent as overlapping notes (partly built: the overlap exists and
  the sample source follow uses it; still to do: the `_` mark, and one
  note for the same pitch in two voice layers).
- `euclid(hits,steps)`.
- Repeat signs `|%n ... %|`.
- Octave annotations `+8va`, `-8va` and `loco`; the `home_octave`
  setting removed.
- Inversions, `[1stInversion]` and so on.
- `grouping` on a meter.

**Decided for the marks and the dynamics line** (*Jim, 11 Nov 2026*). These
fill what the sections above leave unsaid. The words are musical:
**crescendo** is `<` on a dynamics line and **diminuendo** is `>`; the
word "wedge" is not used for them.

- **Straight line.** A crescendo or diminuendo moves the level in a
  straight line in time over its written length, from the level before
  it to the level written after it. A sounding element takes the level at
  the moment it starts, rounded to the nearest whole velocity. Example:
  halfway through a crescendo from `p` (40) to `f` (88) at a normal
  velocity of 64, the level is 64.
- **The author writes the target.** A crescendo or diminuendo ends where
  the next level mark begins (`pp`, `p`, `mf`, `f`, `ff`), which may be
  bars later (a crescendo over two bars is `<:w` for two bars, then the
  target level). A crescendo or diminuendo with no level written after
  it, or followed directly by another crescendo or diminuendo, is refused
  with a message that says to write the target level.
- **No level before it.** A crescendo or diminuendo at the start of a
  line starts from `mf`, the normal velocity.
- **A mark on a note inside a crescendo or diminuendo** (`>`, `>>`, `<`,
  `<<`) is measured from the level the line has reached at that note.
- **A studio `accent` or `marcato` velocity is absolute.** It overrides
  whatever level is in force, even under `p`.

**To confirm before the item is built:**

- The code names for the accent levels of `<` and `<<`: built as `Soft`
  and `Softer`.
- Whether `:euclid(3,8)` may carry marks like any other element:
  proposed yes.

**To check against the Co-work decisions** (choices Claude made when it
built `sample_source`; neither is stated above): a `sample_source` line
may not hold a rest, and a source pairs with the rhythm line of its own
voice layer (`voice_layer 2 sample_source` with the layer 2 rhythm line).

**Built:**

- The `trp` and `fit` groups (`crates/song_text/src/rhythm_bar.rs`).
- A tie with no note after it, at the end of a rhythm pattern or of a
  song line, is refused (`crates/vocabulary/src/rhythm_pattern.rs`). This
  was already so before 11 Nov 2026.
- `sample_source` lines, rhythm-only elements (`:q`) and track and hold,
  at the track's tempo with one play head (11 Nov 2026;
  `crates/song_text/src/rhythm_bar.rs`, `crates/vocabulary/src/song.rs`,
  `crates/player/src/sample_source.rs`; tested by
  `crates/song_text/tests/sample_source_plan.rs`, and `songs/Sample_source.song`
  to hear). When the source changes inside a sounding element, the new
  notes start before the old ones end, with no new gate (a legato note
  off, `crates/player/src/schedule.rs`); a pitch both chords play is held
  through, not struck again. The `_` legato mark and one note for the
  same pitch in two voice layers still use none of this. The LilyPond
  export refuses a song with a sample source.
- Fractional tempos (11 Nov 2026): `tempo 112.5 beats/min`, up to three
  decimals, held exactly as whole milli-beats; a tempo that does not make
  a whole number of milli-quarters per minute is refused
  (`crates/vocabulary/src/tempo.rs`, `crates/song_text/src/song_file.rs`).
- `marcato` in place of `strong`, and `velocity normal` 64 when not
  written (11 Nov 2026; `crates/song_text/src/studio_file.rs`,
  `crates/player/src/studio_configuration.rs`).

- Relative velocities and the marks `<` and `<<` (11 Nov 2026): a mark
  on a note moves the level in force 3/8 or 3/4 of the way toward 127 or
  1, rounded to the nearest whole velocity; studio `accent` and
  `marcato` are absolute overrides; `<` and `<<` draw nothing
  (`crates/vocabulary/src/midi_values.rs`,
  `crates/player/src/studio_configuration.rs`,
  `crates/song_text/src/rhythm_bar.rs`). The levels of the `dynamics`
  line are not built.

**Tested by Jim on the real studio (Nov 2026)**, recorded in
`crates/song_text/examples/song_play_test/explainer.md`: the Demo's
count-in and beat velocities (127 and 80 on the studio's accent and
normal), the Keys and the Bass, checked on MIDI Monitor and by ear. The
soft marks `<` and `<<` were checked on the Beat track with normal 80:
beat 1 sent velocity 50 and 21 (the player's log and the receiver's MIDI
log agree), the unmarked beats 80. The sample-source song is built and
unit-tested but not yet recorded as heard.

**Where the code lives.** The vocabulary crate holds the musical types
(`RhythmPattern`, `VoiceLine`, `Song` and their parts). The studio
configuration is not music, and A-1 keeps devices out of the vocabulary,
so its type (`StudioConfiguration`) lives in the `player` crate, which
configures ports for a song (*decided*, Q18), together with the check of
what a song needs against what a studio gives. The workspace crate
`song_text` holds the parser (and, later, the writer) for all four kinds of
file, by hand, with no new external crate (A-3); it depends on `player`
only to build a `StudioConfiguration` from a `.studio` file. Apply is authoring logic, not the player's, and
where it lives is settled when it is built.

## 9. Open questions

Every open question in this document is here, labelled OPEN_QUESTION.
When a question is decided,
its block is replaced by the decision, written into sections 1 to 8.

Each block gives the question, its context, why it matters, the options
(with a recommendation, which is not used until Jim confirms it),
and where and when it was raised.

On 11 Nov 2026 Jim answered Q4, Q13 to Q17, Q20, Q21, Q23 to Q27 and
Q29 to Q33. Those decisions are now in sections 1 to 8, marked
"11 Nov 2026".

**OPEN_QUESTION Q19:** How does a song drive a queued device such as the
Korg SQ-1? *Required, not yet designed.*
- Context: a cue that only starts it (`| "Start SQ-1":w | r |`) is not
  enough. The device is clocked, so the song must make explicit the
  Timing Clock it sends to that device in relation to the device's
  documented step resolution (the SQ-1 has six settings: quarter, 8th or
  16th note, at SYNC ×1 or ×2; see the router project's
  `SQ-1 Observations.md`). Cues must be able to start and stop it, once
  or looped for n loops. At high tempi the device is an oscillator
  (22000 q/min was reached on 4 Nov 2026).
- Why it matters: until it exists the SQ-1 has no place in the song
  tests.
- Options: none proposed yet. The device knowledge, including the step
  resolutions, belongs in a device profile (EFS-4.6).
- Status (Jim, 11 Nov 2026): left open so that development is not
  stalled. Cue tracks lack enough definition to be implemented and will
  be taken up as a whole. Limiting a cue to chosen passes of a repeated
  section is part of that work.
- Raised: Jim, 5 Nov 2026.

**OPEN_QUESTION Q28:** How does a source line run at its own tempo?
- Context: Jim decided on 11 Nov 2026 that every source line (a
  `sample_source` line, a `dynamics` line) has its own play head. By
  default it moves at the track's tempo. A source line may have its own
  tempo while keeping the track's time signature, so only the position
  of its play head differs. Its tempo could also rise or fall gradually,
  and tempo marks could apply to it. This is step 3 of the order of the
  work in section 8, "Build order, tests and where the code lives".
- Why it matters: a source that drifts against the rhythm changes which
  chord or level each rhythm element meets, which is the variation Jim
  is after. The player needs rules before it can be built.
- To decide:
  - how the line states its tempo, for example
    `line "Keys"  sample_source  tempo 90 beats/min`;
  - what the play head does when it reaches the end of the section's
    bars (return to the section's first bar is the obvious candidate);
  - what it does when the section repeats or the next section starts:
    start again, or carry on from where it is;
  - how a gradual change of its tempo is written.
- Options: none chosen. Jim takes this up after every item of the list
  "Decided on 11 Nov 2026 and not yet in the code" in section 8 is
  built; shifting play heads are explored then (Jim, 11 Nov 2026).
- Raised: Jim, 11 Nov 2026.

**OPEN_QUESTION Q34:** How does a track send pitch to a device that
gates again on every note on, so that a change of source note inside a
sounding element is heard as legato?
- Context: legato is built as overlapping notes (the new note on, then
  the old note off). On 12 Nov 2026 Jim heard the Disting NT gate again on
  every change of the sample source, so on that receiver the overlap is
  not legato. MIDI 1.0 cannot change the pitch of a sounding note without
  a note on, and the receiver decides whether a note on while another is
  held fires a gate. Pitch bend would only slide or offset the pitch and
  depends on the receiver's bend range, so Jim does not want it.
- Why it matters: with track and hold (section 5, "Sample sources and
  uncoupled lines") the rhythm must drive the gates and the source must
  drive only the pitch. The `_` mark, a tie, and one note for the same
  pitch in two voice layers need the same thing.
- Direction (Jim, 12 Nov 2026), not yet decided: carry the pitch on
  control change (CC) numbers, and use the note on and note off for the
  gate only, as a modular system keeps pitch and gate apart.
  - The studio says per track that its pitch goes on CCs; the song
    does not change.
  - CC numbers start at 60 (a mono track uses 60, a chord up to four,
    60 to 63), clear of Jim's controller, which uses up to 52.
  - A chord with fewer than four notes sends the root on the CCs it does
    not use.
  - A per-track delay in the studio, about plus or minus 20 ms, between
    the CC and the note on, so the gate never opens on the previous
    pitch (which direction is positive is not fixed).
  - One CC step is one semitone (the value is the MIDI note number).
  - The note on still carries the element's first pitch, so a device that
    listens only to notes plays the rhythm.
- Findings (Jim, 12 Nov 2026, with `held_notes_test`: Bass note on at
  beat 1, a second note on at beat 3 with the first still held, both
  note offs at the end of the bar; logged 12 Nov 2026 19:05 local time):
  - FH-2 making gate and CV, into an analog voice (CV to the oscillator,
    gate to the envelope, envelope to the amplifier): a hard gate at
    beat 1 and a legato change of pitch at beat 3. The gate stayed high.
    So the held-note sequence works with a MIDI to CV converter.
  - Disting NT, Macro Oscillator 2, driven by that same CV and gate from
    the FH-2: the note at beat 1 sounds, and there is no pitch change at
    beat 3. Disting NT, Poly FM driven by CV and gate: a gate at beat 1
    only and no pitch change at beat 3. The later entry below shows
    Poly FM following the pitch, so the cause here is not known.
  - Disting NT under MIDI control: Macro Oscillator 1 and Poly FM both
    gate their envelopes at beat 1 and again at beat 3.
  - 12 Nov 2026 19:17 (local time, Jim): the player's own sequence,
    `song_play_test` with `songs/Legato_test.song` and `songs/Demo.studio`
    (note on at beat 1; at beat 3 the second note on and then the first
    note off at the same instant; the second note off 5 ms before the bar
    ends), FH-2 CV and gate into the Disting NT's Poly FM with MIDI
    switched off in Poly FM: the pitch change at beat 3 is a clear
    legato note, like an analog oscillator. Under MIDI control the same
    song gives two gates.
  - So the same FH-2 and the same NT algorithm gave no pitch change with
    `held_notes_test` (both keys held until the end of the bar) and a
    legato change with the player's sequence (the first key released at
    the change). Jim ran it across several NT algorithms and does not
    think the setup differed. The cause is not known. A spurious gate
    from the FH-2 when the first key is released was considered and
    judged very unlikely (an envelope triggers only on a rising edge, and
    a release would not make one), so it is not being pursued.
  - Every NT algorithm tested so far emulates a Yamaha or Mutable
    Instruments instrument. Jim will keep testing across NT algorithms to
    see whether the behaviour is common, and can ask Os at Expert Sleepers
    directly. Still to test: the System 35's MIDI converter and the ARP
    2600 (whose dual mono mode plays a second note legato).
  - Known to work on the NT side (Jim, 12 Nov 2026): Poly FM transposes a
    held note from a CC (a CC mapped to its transpose input, tested with a
    Launch Control CC). Any NT value input can be mapped to a CC, so the CC
    route is simple on the receiver; which algorithm parameters can take a
    pitch CC exactly, one step per semitone, is still to be checked.
- To decide: how each receiver turns the CC into pitch (the Expert
  Sleepers NT, EX and FH-2 mappings, which Jim is checking); the CC to
  1 V/octave conversion an analog oscillator needs somewhere in the path
  (exactly one semitone per CC step, no slew); and whether the delay is
  one setting per track or one per CC.
- Options: Jim's direction above. The receiver test (steps the pitch CC
  one semitone at a time and is checked on a tuner or quantizer) comes
  before the build. A plugin for the Disting NT that makes legato from
  overlapping notes is a possible alternative for that receiver. A
  measurement of overlapping notes, CC 68 and CC 126 was proposed and not
  taken up (Jim, 12 Nov 2026).
- Raised: Jim, 12 Nov 2026; recorded 12 Nov 2026 16:18 (local time).
- Status (Jim, 12 Nov 2026 19:26 local time): on hold. The result so far:
  the FH-2 with an analog voice works with the player's legato, and so
  does the NT's Poly FM on CV and gate; the NT's own MIDI mode gates again
  on every note on. Nothing is to be built until the further tests are in.

### Deferred (Jim, 11 Nov 2026)

These are decisions to wait, not open questions. Each becomes its own
OPEN_QUESTION when its design starts.

- **Navigation marks** (D.C., coda and the like): a deferred capability.
- **Pressure and pitch bend:** deferred. They need a focused discussion
  of control tracks, cue tracks and continuous-value tracks.
- **More functions** from Strudel (repeat a step, play a group faster,
  weights): can be added later as features. `/` and the `fit` functions
  cover them for now.
- **The Disting NT losing a touching note off and note on** (was Q27):
  long closed. It is reopened only if the problem occurs again.

### Settled here, to be moved into sections 1 to 8

- **Q11.** Retrigger gap. *Settled 5 Nov 2026.* Two notes that touch put a
  note off and a note on at the same instant, and the Disting NT, fed over
  USB, loses the gate of the second note. In 2-minute runs of 480 notes
  (`retrigger_gap_test`), 5 ms and 20 ms lost none and 1500 µs lost 17.
  The ALSA output therefore sends the note off of any note that touches
  the next note on its track (a bass line or a chord alike, any pitch) at
  least 5 ms before it, and the note on stays on its beat. The studio's
  `retrigger_gap` can raise that lead above 5 ms as a stopgap. A legato
  mark is a connected gate, with no gap; it is not built yet.
- **Q22.** `#` is both the comment sign and the sharp. *Settled (Jim,
  8 Nov 2026):* a comment is a whole line starting with `#`, or the rest
  of a line after `---`; every other `#` is a sharp (section 2). Found
  writing BWV 610, whose sharps were cut off.

## Appendix A — A chord's intended place inside a library bar

Withdrawn on 11 Nov 2026. A progression carries no rhythm or timing
(section 7, "The libraries (`.library`), and apply"), so there is no
intended place to write, and the `@` spelling is not used. Git keeps the
earlier text.

## Appendix B — What Strudel's mini-notation offers

Read on 4 Nov 2026 from strudel.cc (mini-notation, cycles, time modifiers
and voicings pages), summarised in our words. Strudel is a pattern
language for live coding; it has no bars.

| Subject | Strudel | This notation | Take |
|---|---|---|---|
| Unit of time | A cycle; the steps written in it share it equally | A bar of the song's meter | Keep bars: the counter, cues and ties all need them |
| Grouping | `[ ]` fits a group into one slot | `trp (...)`, `fitQ(...)`, `fitBar(...)` | Borrowed, with a named length so a group can span a note or a bar |
| Unequal lengths | `@` weight: `a@3 b` is 3 to 1 | Dotted values, ties, and the proportions inside a `fit` | Printed-music spelling, which the rhythm line draws |
| Rest | `~` | `r` (`r:q`) | `~` is a tie here |
| Repeat | `*2` plays a group twice as fast; `!` repeats a step | A rhythm repeats over a bar range | Same need, different tools |
| Euclidean rhythms | `bd(3,8)` spreads 3 hits over 8 steps | `euclid(3,8)`, the steps filling the bar | Adopted (Jim, 11 Nov 2026) |
| Chords | Symbols like `Am7`, with `voicing()` choosing notes that move least | Chord names carried by elements, voiced plainly unless the musician changes them (A-9, EFS-5.5) | Do not adopt automatic voicing |
| Exactness of time | Not stated on the pages read | Whole ticks | Ours |

What the pages did not show: any form for triplets, dotted values or ties
across a cycle boundary; polymeter (`{ }`, `%`); and what `_` means, which
appears in an example without explanation. From general knowledge, not
verified here, `_` holds the previous step and `{ }%` is Tidal's
polymeter. The page also contradicts itself about how long the default
steps last (a "quarter second" against a 2-second cycle), so I would not
rely on its timing statements.

## Appendix C — Instruments, clefs and transposition

For `musician_voice` tracks (later). From common practice. "Sounds" is the
interval of the sounding pitch from the written pitch. These rows are
examples only (*Jim, 11 Nov 2026*): the clefs come from standard
notation, and an instrument's label is typed freely by the musician.
Claude found no error in the table on 11 Nov 2026, but did not check it
against a named reference.

| Instrument | Usual clef | Sounds |
|---|---|---|
| Violin | treble | as written |
| Viola | alto (treble for high passages) | as written |
| Cello | bass (tenor, treble for high passages) | as written |
| Tenor trombone | bass (tenor for high passages) | as written |
| Flute | treble | as written |
| Trumpet in B♭ | treble | a major second lower |
| Clarinet in B♭ | treble | a major second lower |
| Alto saxophone in E♭ | treble | a major sixth lower |
| French horn in F | treble | a perfect fifth lower |
| Classical guitar | treble | an octave lower |
| Double bass, bass guitar | bass | an octave lower |
