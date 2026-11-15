# Live Lead Sheet — Idea: a controller for the sample source

Revision 3 — 15 Nov 2026 (controller lines may have any number of bars, section 8.4; SC12 added; SC11 left open)

**Status: an idea, not a decision.** Nothing here is built, and nothing here
changes the notation yet. What Jim has stated is labelled *Jim's idea*; what
Claude proposes is labelled *proposal*. Open questions are in section 9,
tagged OPEN_QUESTION. Terms (bar, beat, tick, score position) are those of
`Musical Time and Clock.md`. It sits beside `concept_idea.md`, whose second
capability, decoupled shaping of the music, this is a concrete form of.

## 1. The idea in brief

*Jim's idea.* Today a track can be written on two lines. A `sample_source`
line holds the chords or notes, and a rhythm line beneath it holds only
lengths: the rhythm is the gate, the source is the voltage being tracked
(Song Notation, section 5, "Sample sources and uncoupled lines"). The
source is read at the same place the rhythm is, bar for bar, so each pass
of a section sounds the same.

The idea is a third voice for the track: a **controller** that decides
*which part of the source the gate reads*, and *how that reading is
transposed*, while the section runs. The rhythm still anchors the rhythm.
The source still holds the harmony and its progression. The controller
breaks the sameness of repeats, both the repeats of a section and the
repeats of a group of bars (the `|% ... %|` signs, in Song Notation, section
5, "Sections").

The controller has two independent dimensions, like two control voltages:

- **Horizontal: where the source is read.** Advance, hold, go back or jump
  by beats, bars or steps, so the same rhythm meets different chords.
- **Vertical: how the reading is transposed.** By a chromatic shift (an
  exact number of semitones, the key ignored) or a diatonic shift (a number
  of steps of the section's scale, the key respected).

Rhythm, harmony and variation then each live on their own line, and the
progression is still honoured because the source is still what is read.

## 2. Words used here

| Word | Meaning here |
|---|---|
| source | a `sample_source` line: the chords or notes, with no lengths |
| rhythm line | the plain line below a source: lengths only (`:q :e :h`) |
| read position | the place in the source that a gate reads when it starts |
| controller | the line or lines that move the read position and transpose what is read |
| step | one element of the source (one chord or note as written) |
| pass | one play of a section, or of a repeated group of bars |
| chromatic shift | every pitch moves by the same number of semitones; the key is ignored |
| diatonic shift | every pitch moves by the same number of scale steps of the section's key; the key is kept |

**A word to check.** Jim's note called the key-ignoring shift
"enharmonic". In music theory *enharmonic* means the same sounding pitch
written two ways (C♯ and D♭), so it names a spelling choice, not a kind of
shift. The usual terms for the two shifts are *chromatic* (also *real*
transposition: the intervals of the model are preserved exactly) and
*diatonic* (also *tonal*: the notes are moved within the key). The teaching
sources do not draw the line identically, since "chromatic" is sometimes
defined by the interval moved and "real" by the intervals kept, so this
document defines each shift by what the program does (table above) and
uses chromatic and diatonic throughout. Enharmonic spelling does come back
in a real place: after a chromatic shift of D2 A2 F2 D2 up one semitone,
the third note is the same key on the keyboard whether it is written F♯ or
G♭. The player sends the same MIDI note either way; the choice only matters
to the staff, and belongs to the spelling code (`note_spelling`).

## 3. Where it fits in the model we already have

Today the read position is the score position: the source is read where
the rhythm is. The controller generalises that with one function, so the
existing behaviour is the special case with no controller:

```
read position = a function of (score position, the controller's commands)
pitch read    = the source's pitch at the read position,
                shifted by the controller's current shift
```

Everything else stays as built. The rhythm element is still the gate. An
element still plays what the source holds when it starts, and still follows
a change while it sounds (track and hold, with legato sent as overlapping
notes). A rest still closes the gate. A source with no controller line
reads exactly as it does today.

*Proposal.* The controller is **resolved when the song is planned**, not
while it plays. For each gate the planner works out the read position and
the shift, the way it already writes repeats out in play order. That keeps
three properties the project already values: the music for any bar is
known ahead, so the strip can show the notes that will actually sound
(EFS-1.6); a song plays the same way every time; and nothing about the
controller needs the real-time clock. A live controller (a knob or a MIDI
input) would be a different thing, listed under "Later" in section 8.

*Relation to open question Q28* (Song Notation, section 9: how a source
line runs at its own tempo). Q28 asks how the source's play head moves, and
what it does at the end of the section's bars and when the section repeats.
This idea is a way to answer it: the play head can run at its own rate
because a controller moves it, and "what happens at the end" becomes a
property of the controller (section 5, "At the end of the source"). The
two should be settled together.

*Relation to transforms* (Architecture, A-10, transforms scoped in time).
A transform changes a song from outside, for a chosen section and repeat.
The controller is part of the composition, written in the song file and
played from it. They can share the code that shifts a pitch diatonically
or chromatically.

## 4. The horizontal dimension: where the source is read

### 4.1 The difficulty Jim raised

The source's elements share each bar evenly: `D2 A2 F2` in a 4/4 bar is
three steps, each one and a third quarter notes long. "Advance one beat"
does not land on an element boundary. Two honest readings exist, and both
are useful, so the controller offers both as different units:

- **Advance in time** (`beat`, `bar`). The read position moves by that much
  source time. A gate then reads whatever step covers the new position.
  Landing in the middle of a step is fine: it reads that step, as a gate
  starting in the middle of a step does today.
- **Advance in steps** (`step`). The read position moves to the start of
  the step that many steps along, whatever its length. This is what a
  sequencer with a step counter does, and it is independent of how the bar
  is shared.

*Proposal.* Support `bar` first, because a bar is the unit the harmony is
written in and it never lands mid-chord, then `step`, then `beat`. The
beat is the least safe unit for a source with uneven sharing, and can wait.

### 4.2 What the controller can say

*Proposal.* A small set of moves, in the existing style: one word each.

| Word | Meaning |
|---|---|
| `+n` before a unit, e.g. `+1bar` `+2step` `+1beat` | move forward by n of that unit |
| `-n` and the unit | move backward by n |
| `=` | hold: the read position stays where it is |
| `base` | back to the source as written: read where the rhythm is, as today (the default) |
| `@n` and `bar` or `step`, e.g. `@3bar` | jump to the start of source bar or step n (counted from 1) |

The controller starts at **base**, so a source with no horizontal
controller line behaves as it does today. The first command takes the read
position over; `base` hands it back. (`~` is not used: it is the tie mark.)
Moves are **relative**: `+1bar` moves from wherever the read position is,
so repeated moves accumulate; `@n` is the absolute form. Shifts (section
6) are the other way: `d+1` *sets* the shift to one scale step, however
often it fires. Whether the read position carries on from one pass to the
next, or starts each pass at `base`, is open (SC11).

### 4.3 Directions, as hardware sequencers have them

Hardware step sequencers name the common ways to walk a pattern: forward,
backward, pendulum (forward then back, with or without repeating the end
steps) and random. These are all expressible as a repeated move (`+1step`,
`-1step`) with an end rule (section 5), and a short list of named
directions could be offered as shorthand later:

| Direction | The same as |
|---|---|
| forward | `+1step` on each gate |
| backward | `-1step` on each gate |
| pendulum | forward to the last step, then backward to the first |
| random | a step chosen at each gate (needs a seed; see section 9) |

### 4.4 Own tempo and meter

The controller can also change the *rate* at which the read position
moves: `+2bar` on each bar plays the source at twice the speed, `+1bar` on
every second bar at half the speed. That is Q28 by another route (a source
with its own tempo), and it needs no separate tempo for the source. A
source whose length is not the section's length (a three-bar chord cycle
under a four-bar rhythm) is a loop, and is what the end rule of section 5
describes.

## 5. At the end of the source

When a command would take the read position past the last bar of the
source, or before the first, something has to happen. Three rules are
common in sequencers:

| Rule | What happens |
|---|---|
| wrap | carry on from the other end, as a loop does |
| clamp | stay on the last (or first) step |
| bounce | turn round and go the other way (a pendulum) |

*Proposal.* **Wrap** is the default, because it matches how a loop of
chords is understood and cannot fail. The rule is a setting of the
controller line, not of each command. A source that is a loop of its own
length against a longer rhythm is then just wrap.

## 6. The vertical dimension: how the reading is transposed

### 6.1 The two shifts

*Proposal.*

| Word | Meaning | Chord `I` in C major, shifted by 1 |
|---|---|---|
| `c+n` | chromatic: n semitones up (`c-n` down) | C♯ major; the progression leaves the key |
| `d+n` | diatonic: n scale steps up (`d-n` down) in the section's key | `ii`, D minor; the chord changes quality as the key requires |
| `o+n` | n octaves up (the same as `c+12n`) | the same chord, higher |
| `base` | back to the source as written: no shift | `I` |

For a bass line, take D2 A2 F2 D2 in C major:

| Shift | Result | Note |
|---|---|---|
| `d+2` | F2 C3 A2 F2 | each note two scale steps up; stays in C major |
| `c+2` | E2 B2 G2 E2 | two semitones up; here it also happens to stay in the key |
| `c+1` | E♭2 B♭2 F♯2 (or G♭2) E♭2 | one semitone up; leaves the key |

A chord shifted diatonically keeps the rule "the chord that the key gives on
that degree", so `I` shifted by `d+1` is `ii`, and a chord written as
notes, `[C3 E3 G3]`, moves each note by the same number of scale steps and
comes out as the same chord on the new degree.

### 6.2 Why both

The diatonic shift is how a melody or a bass line is moved in tonal music (a
tonal sequence); it keeps the harmony inside the key and is the safe
variation. The chromatic shift is the exact-interval move (a real
sequence), which a modular system does as a plain offset on the pitch
voltage. It is the bold variation, and it is what moves a whole progression
to a new key. Live 12 shows the same split in a product: the Chord device
adds fixed semitone intervals and does not know the key, while the Pitch
device can work in scale degrees when set to the current scale (see
"Prior art", section 7).

### 6.3 What the shift applies to

The shift applies to what the gate reads, after the read position is
found, so the two dimensions combine without either needing to know about
the other. A held element that follows a source change (track and hold)
follows a shift change the same way.

## 7. Prior art

What other tools and traditions do, and what this borrows. The searches
behind this section were short; claims marked *not confirmed* were not
found in the source.

| Source | What it does | What we take |
|---|---|---|
| Hardware step sequencers (Eloquencer, Behringer STEPS, Audio Damage Sequencer 1, Reason's Thor sequencer) | Direction modes: forward, backward, pendulum (with or without repeating the end steps), random. The Eloquencer sets them per track and pattern | the direction names and the end rules (sections 4.3 and 5) |
| Noise Engineering Mimetic Sequent | A *Beat* input that advances the current step: the sequencer moves only when told to | "advance only on command" (section 4.2) |
| Elektron sequencers, trig conditions | A step can play only on some passes of the pattern, such as `1:2` (the 1st, 3rd, 5th pass), or depending on a neighbouring track | per-pass variation (section 8.1) |
| Strudel and TidalCycles | `every n` applies a change on every nth cycle; `rev` reverses a pattern; `palindrome` reverses every other cycle; `iter n` shifts the start by one nth each cycle; `degradeBy` drops events at random. The `<a b c>` bracket alternates one value per cycle (*not confirmed* in the pages read) | per-pass alternation with `<...>`; `rev`-like and `iter`-like moves as combinations of our words |
| Ableton Live 12 | The Chord device works in semitones and does not know the key; the Pitch device works in scale degrees when set to the current scale; Scale pulls notes into a key | the two shifts, and the evidence that musicians use both |
| Music theory: tonal and real sequences | A model repeated up or down by scale steps (tonal, diatonic) or by exact intervals (real, chromatic) | the definitions of the two shifts |
| Modular practice | Pitch and gate are separate voltages; a clock advances the step; an offset or quantiser moves the pitch; a sample and hold reads the pitch at the gate | the whole framing: this program's rhythm is the gate, the source is the voltage |

Sources: [Perfect Circuit, on sequencer direction](https://www.perfectcircuit.com/signal/learning-synthesis-sequencers-2);
[Reason Thor sequencer pendulum modes](https://docs.propellerheads.se/reason9/Thor.31.6.html);
[Winter Modular Eloquencer](https://reverb.com/item/15237944-winter-modular-eloquencer-sequencer);
[Noise Engineering Mimetic Sequent manual](https://manuals.noiseengineering.us/ms/);
[Strudel time modifiers](https://strudel.cc/learn/time-modifiers/);
[Strudel conditional modifiers](https://strudel.tidalcycles.org/learn/conditional-modifiers);
[Elektronauts, trig conditions](https://www.elektronauts.com/t/ot-trig-conditions-trc-thread/44828?page=19)
(forum posts, not the manual, so check the manual before relying on the
detail);
[Ableton Live 12 MIDI effect reference](https://www.ableton.com/en/live-manual/12/live-midi-effect-reference/);
[Ableton scale awareness, Isotonik](https://isotonikstudios.com/scale-awareness-in-ableton-live-12-ned-rushs-playful-deep-dive-via-midi-devices-and-clip-modes/);
[Real and tonal sequences](https://intmus.github.io/inttheory20-21/18-tonal-sequences/a2-tonalsequences.html);
[Sequence (music), Wikipedia](https://en.wikipedia.com/wiki/Sequence_(music)).

## 8. A simple notation

### 8.1 Proposal

The project already writes control information in **rhythm notation**: cues,
tempo marks and dynamics are lines of elements, and the element's place
says when it fires (Song Notation, section 5, "Cues, in rhythm notation").
The controller follows the same rule, so its commands are placed against
the rhythm, and no one counts bars across repeats.

Two optional lines may follow a source, one for each dimension, because
keeping the dimensions on separate lines is the decoupling Jim describes:

```
line "Bass" sample_source  | D2 A2 F2 D2 | G2 C3 |
line "Bass" source_move    | base:w      | base:w |       --- horizontal
line "Bass" source_shift   | base:w      | d+1:w  |       --- vertical
line "Bass"                | :q :q :q :q | :h :h |        --- the rhythm
```

An element is **what to do, then its length**, as everywhere else in a
line (`d+1:w`: a diatonic shift of one step, for a whole note). The command
fires where the element starts and holds until the next one. Existing
rules carry over without change:

- a length left off is the one before it;
- `/` does again what the element before it did, so `| +1bar:w / / / |`
  advances one bar in each of four bars;
- the bars of the source and of the rhythm line must add up to the
  section, like any line; a controller line is the exception (section
  8.4).

### 8.2 Varying from pass to pass

Variation across repeats is the point of the idea, so an element may hold a
**list of whole commands**, in angle brackets, used one per time the
element fires, and starting again from the first when the list runs out
(the Strudel `<a b c>` idea):

```
line "Bass" source_shift   | <base d+1 d+2 d+1>:w |   --- 4 passes, then round again
```

Over a section written with `repeat 4`, the four passes play the written
progression (`base`), then a step up, then two steps up, then a step up
again. The first pass is `base`, so it is as written. The
same notation works inside a `|% ... %|` group of repeated bars, because
the list is counted by how many times that element has fired, not by what
kind of repeat caused it. Whether that is the right count is open (Q-SC1).

An Elektron-style condition ("only on the 3rd of 4 passes") is a list with
`base` in the other places: `<base base d+2 base>`.

### 8.3 More examples

The same source and rhythm (4/4, source `| I | IV | V | I |`, a whole note
in each bar), read in four ways. A command takes effect where its element
starts, before the gate in that bar reads, and the read position starts on
bar 1 of the source:

```
no controller                         plays  I   IV  V   I
source_move  | +1bar:w / / / |        plays  IV  V   I   I     (bar 4 wraps to bar 1)
source_move  | -1bar:w / / / |        plays  I   V   IV  I     (bar 1 wraps back to bar 4)
source_shift | d+1:w / / /   |        plays  ii  V   vi  ii
```

A bass line moved up an octave on the second pass only:
`source_shift | <base o+1>:w |` under a section with `repeat 2`.

### 8.4 A controller line may have any number of bars

*Jim's idea, 15 Nov 2026.* The rule that every line of a section has the
same number of bars stays for the source and for the rhythm line (and for
any plain line). It is relaxed for `source_move` and `source_shift`, which
may hold any number of bars, one or many, shorter or longer than the
section:

```
section "Verse"  repeat 4                                  --- 8 bars
  line "Bass" sample_source  | D2 A2 F2 D2 | G2 C3 | ... 8 bars
  line "Bass"                | :q :q :q :q | ... 8 bars
  line "Bass" source_shift   | base:w | d+1:w |             --- 2 bars
```

Song Notation (section 5, "Sections") records that a line shorter than the
section, a cycle repeated to fill it, is polymeter, which is deferred, and
that such a line must be marked so that a line left short by mistake stays
an error. A controller line is marked by what it is: the line keyword says
it is a controller, so a short one is never a mistake. This idea therefore
asks for the one case of polymeter that is cheap and has a clear use, and
only for controllers.

*Proposal.* A controller line is a **cycle of its own length**. Its bars
are used in order, and when it comes to its end it starts again from its
first bar, until the section ends. A two-bar shift line under an eight-bar
section alternates its two bars four times. A controller line longer than
the section is cut off where the section ends. When it begins again at the
start of the next pass is SC11, and what it does when its length does not
divide the section's is SC12.

This also gives a second, independent way to vary a repeat without lists.
A controller of 3 bars under a rhythm of 4 bars shifts against it, so the
two come into step only every twelve bars, as two loops of unequal length do
in a modular system.

### 8.5 Why this notation

It is small (one word and a number), it reads in the same order as the rest
of the notation (what, then how long), it needs no new bar structure, it
reuses `/` and the length carry, and it keeps the two dimensions apart. Two
lines for two dimensions also means a song that wants only the vertical
variation writes one extra line, and no more.

*Alternatives considered.* One combined `source_control` line holding both
(`+1bar,d+2:w`), which is shorter to write and harder to read; a property on
the source line itself (`sample_source shift d+2`), which cannot change
during the section; and a full pattern language such as Strudel's
mini-notation, which is far more than needed and would be a second notation
inside the first.

## 9. Open questions

Each question is tagged and dated, and is a working record, as the project's
other documents keep theirs. None is closed.

**OPEN_QUESTION SC1 (raised 15 Nov 2026 15:54):** What does a list in
`<...>` count: the passes of the section, the passes of an enclosing repeat
group, or how many times this element has fired?
- Context: section 8.2 counts the element's own firings, which works in
  every kind of repeat and needs no extra rule.
- To decide: whether the count starts again at each entry of a section
  played again by name, and at each new section; and whether a controller
  of one section can see the pass number of another.
- Options: (a) the element's own firings, restarting never; (b) restarting
  at each play of the section it is in; (c) the section's pass number
  directly (`<>` means "this pass").

**OPEN_QUESTION SC2 (raised 15 Nov 2026 15:54):** Which horizontal units
are built first, and what does landing in the middle of a step do?
- Context: section 4.1. Bars never land mid-chord. Beats do, when the
  source has an odd number of steps in the bar.
- Options: bars only; bars and steps; all three. A move that lands
  mid-step reads that step (as proposed), or snaps to the step's start.

**OPEN_QUESTION SC3 (raised 15 Nov 2026 15:54):** What happens at the
source's ends: wrap, clamp or bounce, and is it a setting of the line or of
each command? Related to Q28's "what does the play head do at the end of
the section's bars and when the section repeats". Proposed: wrap, set per
line.

**OPEN_QUESTION SC4 (raised 15 Nov 2026 15:54):** What is a diatonic shift
of a chord or note that is not in the key (a secondary dominant such as
`V7/V`, a borrowed `bVII`, a chromatic passing note)?
- Options: refuse it at load, with the place; shift its nearest degree and
  keep its alteration; leave it unshifted.
- Note: the loader may not guess silently (project rule); a refusal with a
  message is the safe default.

**OPEN_QUESTION SC5 (raised 15 Nov 2026 15:54):** What happens when a shift
takes a note out of range: past MIDI 0 to 127, or past what the track's
instrument plays?
- Proposed: the planner already knows every note ahead, so it refuses the
  song with the bar and beat, and the amount of shift that is too much. No
  silent clamping.

**OPEN_QUESTION SC6 (raised 15 Nov 2026 15:54):** Randomness: allowed or
not, and if so how is it kept repeatable?
- Context: the strip shows the notes that will actually sound, so a random
  move would need a seed written in the song so that a song plays the same
  way each time. A `random` direction and `degradeBy`-style dropouts are
  the usual forms.
- Options: no randomness in V1; a seeded `random` direction later.

**OPEN_QUESTION SC7 (raised 15 Nov 2026 15:54):** Do the controller lines
apply to coupled lines, for example a line of written notes, or only to
sources?
- Context: the project marks a source on purpose ("marked, not guessed").
  Variation of a coupled line is just as tempting, but it has no separate
  source to read, so a shift would transpose written notes.
- Proposed: sources only at first; a shift on a coupled line later, as a
  transform (Architecture, A-10).

**OPEN_QUESTION SC8 (raised 15 Nov 2026 15:54):** What are the lines called,
and are there two or one? Candidates: `source_move` and `source_shift`;
`read` and `shift`; one `source_control`. The names are those the musician
types and sees at 3 AM, so they should say what they do.

**OPEN_QUESTION SC9 (raised 15 Nov 2026 15:54):** Does a controller belong
to a section instance or to the track? When a section is played again by
name (Song Notation, section 5, "Playing a section again by name"), does it
carry the controller of the section it repeats (as proposed, because it
repeats the section) or start the controller afresh?

**OPEN_QUESTION SC10 (raised 15 Nov 2026 15:54):** How does the controller
meet a change of tempo or meter between sections?
- Context: a separate session is looking at tempo changes between
  sections (a new anchor at each section boundary). The read position is
  a place in the source's own time, counted in ticks of the section's
  meter. A change of meter in the middle of a source that spans sections
  needs a rule.
- Proposed: the controller stays inside one section, and each section reads
  its own source; the question arises only if a source may cross sections.

**OPEN_QUESTION SC11 (raised 15 Nov 2026 16:20):** Does a move command
act from where the read position is now, and does the read position carry
on from one pass to the next?
- Context: `+1bar` is relative and accumulates; `d+1` sets the shift and
  does not. Jim asked (15 Nov 2026) whether `base` on a bar acts the first
  time or only when that bar comes round again. As drafted it acts every
  time the element is reached, the first pass included, and does nothing
  on the first pass because the controller starts at `base`.
- Options: (a) the read position and the shift **restart at `base` at the
  start of every pass**, so each pass is the same function of the
  controller line and the first pass is always as written (proposed);
  (b) they **carry on** from the end of the last pass, so a move keeps
  accumulating across passes, which wanders further each time and is harder
  to predict ahead; (c) the move line is an **offset from where the rhythm
  is** and not a stepper, so `+1bar` means "read one bar ahead of the
  rhythm" until changed, which is easier to follow, but is not the
  "advances only on command" machine Jim described.
- Where and when: raised in chat, Jim, 15 Nov 2026.

**OPEN_QUESTION SC12 (raised 15 Nov 2026 16:40):** What does a controller
line of a different length from its section do at the edges?
- Context: section 8.4. Controller lines may have any number of bars; the
  source and rhythm lines must still match the section.
- To decide: whether the controller cycles to fill the section (proposed),
  or plays once and then holds its last command; whether it starts again at
  its first bar at the start of every pass (the same question as SC11) or
  carries on from where it stopped; what happens when its length does not
  divide the section's (the last cycle is cut off, or the line must divide
  evenly and is refused if not); and whether a controller line with a
  length of 0 bars, or a line that exceeds an agreed limit, is refused.
- Related: the deferred polymeter decision in Song Notation (section 5,
  "Sections"), which would be reopened only for controller lines.
- Where and when: raised in chat, Jim, 15 Nov 2026.

**Later, not questions.** A live controller (a knob, or a MIDI input
controlling the read position or the shift while playing); a gradual change
of the rate (the source speeding up and slowing down); the controller
lines driving the dynamics or the cues; and showing the controller on the
strip as a line of its own beneath the rhythm.

## 10. A suggested order

*Proposal.* Build the part that gives the most variation for the least new
machinery first, and test each part from a canonical song file as the
project does.

1. **`source_shift`, with `<...>` lists.** It needs no read position model:
   the read stays where it is and only the pitch changes. Diatonic and
   chromatic and octave shifts, and the failure messages (SC4, SC5).
2. **`source_move` in bars, with wrap.** The read position becomes a
   function of the controller; `base` remains the default.
3. **Steps**, then **beats**, then **direction words**.
4. **Own rate** and the answer to Q28.
5. Whatever the Concept's decoupled shaping needs after that.

## Revisions

- 15 Nov 2026, revision 1: first draft by Claude Code, from Jim's idea of
  the same day.
- 15 Nov 2026, revision 2: `~` (the tie mark) dropped; `base` is the way
  back to the source as written; lists hold whole commands; SC11 added.
- 15 Nov 2026, revision 3: a controller line may have any number of bars
  (section 8.4); SC12 added. SC11 stays open, with its considerations.
