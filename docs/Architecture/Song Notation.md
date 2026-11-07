# Live Lead Sheet — Song Notation

Revision 13 — 7 Nov 2026 (tempo written in beats; compound meters above 12 refused; "count" replaced by beat and pulse)

**Status: draft for trial.** *Decided* marks what Jim has decided;
everything else is a proposal to try with a working player. The questions
still open are in section 9. Terms (bar, beat, pulse, tick, score position) are
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
| Song (the composition) | `.song` | Tracks, sections, and for each track, bar by bar, its rhythm and the notes each sounding element plays. Cues and tempo marks. Nothing about ports, channels or devices. | Apply, then the musician by hand |
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

## 2. Rules shared by all files

- **Plain text, line by line.** A `#` starts a comment to the end of the
  line. Indentation is for the reader.
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
  are shown (proposed; Q16). Example:

  ```
  14:02:11 WARNING: demo.song is not valid; nothing was applied.
           Issue 1: demo.song line 14: "IV9b" is not a chord. Try IV9 or IV7b9.
           NOT PLAYING: fix demo.song and save it; the program checks it each time.
  ```

## 3. Rhythm notation

A rhythm is one to eight whole bars of one time signature, written as
note values and rests.

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
  live. Here a function is evaluated once, when the song is written or
  loaded, and the song keeps and plays the fixed result. More functions
  can be added later without changing the player. Which ones is open.

**Ties.** `~` joins an element to the one after it into a single note of
the combined duration. It may stand before a barline:

```
rhythm "Anticipation"
  meter 4/4
  | hr qr er e~ | q hr. |
```

The eighth on the "and" of beat 4 is held across the barline into
beat 1 of the next bar: one note an eighth plus a quarter long, a dotted
quarter. The rhythm line draws the tie across the barline (EFS-1.5). A tie
belongs to the rhythm, so every voice track can use it.

**Marks.** Written after the letter, the dots and `r`, and before a tie.
They apply to a sounding element (a fermata may also stand on a rest). The
spellings are proposed; what each does is the decision.

| Mark | Meaning | In the MIDI | On the staff |
|---|---|---|---|
| `>` | accent | note velocity is the device's *accent* level | accent sign |
| `>>` | strong accent | the device's *strong* level | marcato sign |
| `'` | staccato | sounds for half its written duration | dot |
| `_` | legato: joined to the next note | no retrigger gap before the next note | slur |
| `^` | fermata | nothing in V1 | fermata sign |

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

What a sounding element carries follows a colon:

- **Poly voice** (a track that can play chords): a chord name in the
  section's key (`ii7` in C major is D minor seventh), voiced plainly: in
  close position, root position, with the root at or above the track's
  home octave (section 5). Or notes: `[D3 F3 A3 C4]`. A chord name followed
  by notes (`ii7[F3 A3 C4]`) is a chord with its own voicing, for example
  without the root. The plain voicing is EFS-5.5; this spelling of a
  changed voicing is proposed (Q15).
- **Mono voice** (a track that can play one note at a time): one note with
  its octave (`C2`, `F#2`, `Bb1`). Two sounding elements may not overlap.
- **Cue and tempo tracks:** see section 5. A beat pulse needs no section
  of its own: it is a mono voice that writes the same note on each
  element, e.g. `| q>:C4 q q q |`.
- **An element with nothing after it carries the previous element's
  contents** within the line, across barlines (a strum is several elements
  of one chord). The first sounding element of a line in a section must
  carry its own (Q5). Only voice tracks carry forward; every cue on a cue
  control track names its cue.

**Order inside an element** (fixed by the parser): the note value, then
`r` and the dots, then the marks, then `~`, then the colon and the contents
(`q>>:C2`, `e'~:ii7`). Contents end at the next space, except inside `[...]`
or a quoted cue name. A rest carries nothing.

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
  line "Beat"  | q>:C4 q q q |

section "Verse"  repeat 2
  key C major  meter 4/4  tempo 120 beats/min
  line "Beat"  | q>:C4 q q q | q> q q q |
  line "Keys"  | q:I q q q | qr h:ii7 q:V7 |
  line "Bass"  | q:C2 q q q | qr h:D2 q:G2 |
  line "Cues"  | w:"Start SQ-1" | wr |    # fires on each pass: passes are not designed (EFS-4.4)
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
| `musician_voice` | A part composed for a person to read and play, in the clef of that instrument. Sends no MIDI; drawn on its own staff, with marks as interpretation. | Later (the display is beyond V1's one staff) |
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
element, with `>` where the accent falls: `| q>:C4 q q q |`. Its pitch is a
pitch like any other (with `octave_offset` to suit the device, and the
pitch CV it makes at a CV and gate output), and it needs no transport. It
can clock a divider, or voice a metronome in the modular.

Attributes of a voice track:

- `clef` — `treble`, `bass`, `alto`, `tenor`, `percussion`, or `grand`
  (treble and bass braced, for chords that run well below middle C). The
  composer composes in the clef; the music stays on the staff and not on
  ledger lines. The rhythm line always uses the percussion clef (EFS-1.5).
- `lead` — exactly one voice track carries it: its chords drive the key
  wheel and the chord symbols (EFS-3.7).
- `home_octave` — optional. The octave where a plain voicing puts the root
  and where apply puts a bass line's roots. Without it, by clef: treble
  4, alto 3, tenor 3, bass 2, grand 4 (Q14).
- `instrument` — for `musician_voice`: the instrument (Appendix C), which
  sets the usual clef and the written transposition.

**Pitch is stored as sounding pitch.** A `musician_voice` for a trumpet
is stored as the concert pitch that sounds, and drawn a major second
higher, as notation software does. Transposing instruments (Appendix C)
are the only place a written pitch differs from a sounding one.

### Sections

`section "Name"` with `count-in` for the first section if wanted, `repeat
n`, and `key`, `meter` and `tempo` (or carrying on from the previous
section). A key change, a time-signature change and a stepped tempo change
all happen at a section boundary; a change that lasts a few bars is a
short section. The initial tempo is the first section's.

- **Section length** is the number of bars in its lines. In V1 all lines
  in a section have the same number of bars, and a track with no line in a
  section rests. That is how the voices rest in the count-in section
  (D-10). *Polymeter* (a line that is a shorter cycle repeated to fill
  the section) is deferred; if it is added, such a line must be marked as a
  cycle, so a line left short by mistake stays an error.
- **Repeats** are written out in play order by the loader (EFS-1.6). A
  section with no `repeat` plays once.
- **Carrying on.** A section that omits `key`, `meter` or `tempo` carries
  on the one before. The first section must give `meter` and `tempo`
  (nothing is assumed). A section with chords needs a key, written or
  carried; a count-in section needs none. `count-in` is for the first
  section only, one or two bars.
- **Tempo counts beats**, as the meter defines them (`Musical Time and
  Clock.md`, sections 2 and 13): `tempo 120 beats/min` or `tempo 2
  beats/s`, a whole number. No note value is written. In simple time a
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
  - *Open:* tempos that are not whole numbers (`112.5 beats/min`).
- **Meters** are n/d with d = 1, 2, 4, 8 or 16. Compound meters stop at
  12 on top: 15/8, 18/8 and other multiples of 3 above 12 are refused when
  the song is read, with a message, until their beats are decided
  (`Musical Time and Clock.md`, section 2). Other figures above 12 (13/8)
  are simple time.
- **Order in the file.** All `track` blocks come before the first
  `section`; a `line` names a track already declared. Settings (`key`,
  `meter`, `tempo`, `repeat`, `count-in`) may stand on the section line or
  on lines after it, in any order.

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
  rhythm's, so no bar numbers are written: `| w:"Start SQ-1" | wr |`.
- For a `note` cue, the element's written duration is the note's length: a
  short note is a trigger, a held one a gate (EFS-4.3). For the other
  messages the duration does not matter.
- Messages: `note`, `cc`, `program`, and on this track type `start`,
  `stop`, `continue` (a destination for these needs a port only).
- A cue whose destination is `track "Keys"` goes to that voice track's own
  port and channel and is drawn above that track's staff (EFS-4.3, D-10).
- Limiting a cue to chosen passes is not designed yet (EFS-4.4).

### Tempo marks, in rhythm notation (later)

A `tempo_control` track is a line in rhythm notation whose sounding
elements carry a tempo mark:

- `step 100 beats/min` — the tempo becomes 100 beats/min at the start of
  the element.
- `ramp 60 beats/min` — the tempo moves from what it was at the start of
  the element to 60 beats/min by the end of the element. A ramp lasts the
  element's written duration; a tie makes it longer. A ritardando over the
  last four bars: `| w:ramp 60 beats/min~ | w~ | w~ | w |`.

The beats are those of the section's meter, as for `tempo`.

Ramps apply to any stretch of a section (a speeding up in the first bars,
a slowing mid-phrase, a retard at the end after several bars at tempo).
All devices follow, because they follow the clock. Ramps need a design in
`Musical Time and Clock.md` first: its instants are computed from anchors
with a constant tempo each, and a ramp is a continuously changing tempo.
Until then, tempo changes are stepped, per section.

### What the song does not say

Chord symbols and the key wheel read the chords carried by the lead track
(EFS-3.7), at the element where each chord starts. A bass line is free: it
plays the roots in the example, but nothing requires it.

**Not designed yet:** voicing changes beyond the spelling in section 4
(EFS-5.5), key changes between sections (EFS-1.9), limiting a cue to
chosen passes (EFS-4.4), beat grouping for 5/4 and 7/8, minor keys (Q12),
modes and their transitions (they need annotation and visibility; Q12),
navigation marks (EFS-5.10), soft (ghost) notes, pressure and pitch bend,
and the display of `musician_voice` staves.

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
  velocity normal 90  accent 105  strong 120

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
  track's block needs `velocity normal` and a velocity for each accent
  mark its lines use. A block for a
  track the song lacks is ignored, so one configuration serves many songs.
- **`octave_offset`** shifts every note sent, in octaves, to where this
  device places the pitch (the manufacturers disagree where C0 is, and an
  oscillator may sit in a different register). Omitted, it is 0 octaves, a
  harmless default: the note is sent as written.
- **`velocity`** gives the MIDI velocity (1 to 127) for a normal note and
  for each accent mark the song uses (`accent` for `>`, `strong` for
  `>>`). `normal` is required; an accent level is required only when the
  song uses that mark, and the message names the track.
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
  has no key and no timing. When a bar holds two chords, the second
  carries an *intended place*: written without a position it is halfway
  along the bar, and otherwise with `@` (Appendix A). The intended place
  is only a hint to apply.
- **A rhythm** is as in section 3, with its marks.

**Apply** takes a progression, a rhythm, a bar range and a track, and
returns the track's line for those bars. In the editor it is a drag; in
the first tests it is a function run from the terminal. Its rules are
conveniences, and everything it writes can be changed by hand:

1. The bar's first chord goes on the bar's first sounding element, and the
   later elements carry it on. For `| qr h q |` and the chord `ii7` the
   chord goes on the half note. The rhythm's marks are copied onto the
   elements.
2. A later chord in the bar goes on the sounding element nearest its
   intended place, taking the elements in order and never one already
   used. A tie goes to the earlier element.
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
C major give `| qr h:ii7 q:V7 |` for the harmony track and `| qr h:D2
q:G2 |` for the bass. The same progression under `q q q q` gives `| q:ii7
q q:V7 q |`. How the editor snaps to a grid, and what it asks when rule 4
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
   and checks the messages for eight kinds of mistake.
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

Decided questions are in sections 1 to 8. Those that remain, each with
the recommendation that will be used unless Jim says otherwise:

- **Q4.** The tie mark. `~` is the tie in printed notation and LilyPond,
  but in Strudel it is a rest. Keep `~` for ties (our rest is `r`)?
  Recommended: keep.
- **Q5.** An element with nothing after it carries the previous element's
  contents. Keep that shorthand, or write the chord on every element?
  Recommended: keep it, as tempo already carries on (EFS-2.2).
- **Q11.** Retrigger gap. *Settled 5 Nov 2026.* Two notes that touch put a
  note off and a note on at the same instant, and the Disting NT, fed over
  USB, loses the gate of the second note. In 2-minute runs of 480 notes
  (`retrigger_gap_test`), 5 ms and 20 ms lost none and 1500 µs lost 17.
  The ALSA output therefore sends the note off of any note that touches
  the next note on its track (a bass line or a chord alike, any pitch) at
  least 5 ms before it, and the note on stays on its beat. The studio's
  `retrigger_gap` can raise that lead above 5 ms as a stopgap. A legato
  mark is a connected gate, with no gap; it is not built yet. Not proven:
  why the NT loses the pair (its MIDI handling runs every 0.5 ms), and
  devices other than the NT.
- **Q12.** Minor keys: numerals against the natural minor (`i7`, `iv7`,
  `V7`), as the first rule to try? And modes: how a change of mode is
  annotated and made visible is not yet discussed.
- **Q13.** A rhythm assigned to bars 1–12 whose last bar ends in a tie:
  the tie has nothing to join. Recommended: the note ends at the end of
  the range, and the loader warns.
- **Q14.** How to write a chord's intended place inside a library bar
  (Appendix A; the draft uses `@2+e`). And the default `home_octave` by
  clef (section 5).
- **Q15.** The spelling of a changed voicing: `ii7[F3 A3 C4]`, and notes
  alone as `[D3 F3 A3 C4]`.
- **Q16.** When a playing song is edited into error, the last good version
  keeps playing while the errors are shown.
- **Q17.** The spellings of the marks (`>`, `>>`, `'`, `_`, `^`), and of
  cues and tempo marks. What each does is decided; the spelling is to be
  tried.

- **Q19.** Driving a queued device such as the Korg SQ-1 (Jim, 5 Nov 2026;
  *required, not yet designed*). A cue that only starts it
  (`| w:"Start SQ-1" | wr |`) is not enough. The device is clocked, so the
  song must make explicit the Timing Clock it sends to that device in
  relation to the device's documented step resolution (the SQ-1 has six
  settings: quarter, 8th or 16th note, at SYNC ×1 or ×2; see the router
  project's `SQ-1 Observations.md`), and cues must be able to start and
  stop it, once or looped for n loops. At high tempi the device is an
  oscillator (22000 q/min was reached on 4 Nov 2026). The device
  knowledge, including the step resolutions, belongs in a device profile
  (EFS-4.6). Until this exists, the SQ-1 has no place in the song tests.

## Appendix A — Q14, a chord's intended place inside a library bar

Four ways to write the same place. `A` is the draft.

| Where the chord is meant to start | A: pulse + note value | B: spoken counting | C: offset by note values | D: lengths as weights |
|---|---|---|---|---|
| 4/4, pulse 3 | `vi7@3` | `vi7@3` | `vi7@+h` | `I@2 vi7@2` |
| 4/4, the "and" of 2 | `vi7@2+e` | `vi7@2&` | `vi7@+q.` | `I@3 vi7@5` |
| 4/4, the "e" of 2 (a sixteenth in) | `vi7@2+sx` | `vi7@2e` | `vi7@+q+sx` | `I@5 vi7@11` |
| 4/4, a triplet eighth into pulse 2 | `vi7@2+trp e` | no standard form | `vi7@+q+trp e` | `I@4 vi7@8` |
| 6/8, pulse 4 (beat 2) | `vi7@4` | `vi7@4` | `vi7@+q.` | `I@3 vi7@3` |

- **A** numbers the pulses and uses the same letters as the rhythm
  notation, so there is one rule for every case. It read like the counter
  while the counter showed pulses; since 7 Nov 2026 the counter shows
  beats (6/8: `Beat: 2`), so in compound meters A's `@4` and the counter's
  beat 2 name the same place differently.
- **B** is how players say it, and the fastest to type. It covers only
  eighths and sixteenths, it has no form for triplets, and its "e" for a
  sixteenth would clash with the letter `e` for an eighth. In 6/8 the pulse
  is itself an eighth, so "&" would mean something else there.
- **C** needs no pulse numbers, only the distance from the bar's start. With
  letters it can no longer be mistaken for A (`@2` is pulse 2, `@+h` is a
  half note later).
- **D** is the Strudel way (Appendix B): it never says where a change is,
  only how long each chord stays in effect, as a ratio. The weights add up
  to the grid the writer chose (8 eighths, 12 triplet eighths).

Suggested: A as the one exact rule, and `&` accepted as a shorthand for
`+e`. B's other syllables, C and D set aside.

## Appendix B — What Strudel's mini-notation offers

Read on 4 Nov 2026 from strudel.cc (mini-notation, cycles, time modifiers
and voicings pages), summarised in our words. Strudel is a pattern
language for live coding; it has no bars.

| Subject | Strudel | This notation | Take |
|---|---|---|---|
| Unit of time | A cycle; the steps written in it share it equally | A bar of the song's meter | Keep bars: the counter, cues and ties all need them |
| Grouping | `[ ]` fits a group into one slot | `trp (...)`, `fitQ(...)`, `fitBar(...)` | Borrowed, with a named length so a group can span a note or a bar |
| Unequal lengths | `@` weight: `a@3 b` is 3 to 1 | Dotted values, ties, and the proportions inside a `fit` | Printed-music spelling, which the rhythm line draws |
| Rest | `~` | `qr` | `~` is a tie here (Q4) |
| Repeat | `*2` plays a group twice as fast; `!` repeats a step | A rhythm repeats over a bar range | Same need, different tools |
| Euclidean rhythms | `bd(3,8)` spreads 3 hits over 8 steps | None | A candidate for pulse bursts later (EFS-4.7) |
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

For `musician_voice` tracks (later). From common practice, to be checked
against a reference before the table is put into code. "Sounds" is the
interval of the sounding pitch from the written pitch.

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
