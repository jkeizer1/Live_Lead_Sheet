# Live Lead Sheet — Concept Brief

Revision 9 — 30 Oct 2026

## Purpose

An aid for improvising over chord progressions. It shows the musician
what is being played *harmonically* — key, meter, chord names and
voicings — and what is coming next, in standard staff notation the
musician already reads.

It is not a score and not a piano roll. The screen shows the harmony and
its pulse clearly, from arm's length, mid-performance.

## Building blocks

**Chord progression** (library). Chords and where they change, written
in Roman numerals so one progression serves any key. Changes may fall
mid-bar. *Example:* 12-bar blues — I7 ×4, IV7 ×2, I7 ×2, V7, IV7 (or
V7), I7 / V7 turnaround.

**Rhythm pattern** (library). One to eight bars of pulses, written as
note values (quarter, eighth, triplet group, dotted half…). Assigned to
more bars than its length, it repeats; assigned to fewer, it is cut.

**Section.** A progression in a key and meter, with rhythm patterns laid
over its bars, part by part. Key and tempo are set per section, or carry
on from the previous section; a key change happens at a section
boundary. *Example:* in the blues, half notes for bars
1–2, two groups of quarter-note triplets for bars 3–4; in the
turnaround, quarter pulses in bar 11 and sixteenths in bar 12.

**Song.** One or more sections in order. Each section can repeat a set
number of times. A song may loop — *e.g.* a single section, the 12-bar
blues, looping.

**Parts.** What each instrument plays:

- *Chord part* — the chord notes, to a polyphonic instrument, or to a
  device that splits them over several mono voices. A chord part can be
  marked *no root*: the chord is sent without its root, so the root is
  not doubled against the bass in the same or another octave.
- *Bass part* — the roots of the chords, usually on a simple, strong
  rhythm, to a mono voice.
- *Colour phrases* — short phrases (*e.g.* four bars) on their own staff,
  sent to another instrument at chosen points.

One part is marked **lead**: its chords drive the key wheel.

The usual small-studio case: the root clear on one mono voice, the
harmony mostly from one poly voice, everything else adding colour.

**Triggers.** Markers placed at points in the song that make something
in the studio happen — launch a one-shot sequencer, fire a sample, start
a phrase on another device. They may be sparse and irregular. Each
trigger names *what* should happen; a **device profile** says *how* a
given device is made to do it (*e.g.* how to one-shot the Disting NT's
sample player). V1 provides the markers; device profiles and their
details come later.

**When a chord changes between pulses.** The old chord ends exactly at
the change; the rhythm resumes at its next scheduled pulse, with the new
chord. Parts that pulse on the change and parts that don't never sound
two different chords at once.

**Nothing is generated.** Voicings and phrases are chosen by the
musician when authoring. The leader plays exactly what is written.

## What the musician sees

**Song overview.** A strip across the top showing the song's sections
and repeats, with the current position.

**Counter.** Large and plain, in the musician's terms:
`Repeat: 3   Bar: 2   Count: 1`, the count advancing on each beat.

**Parts.** Each part has its own contained area:

- **chord symbols** (**Am7**, **C/E**) at the exact point of each change,
  even mid-beat;
- a **rhythm line** — a one-line staff, drum-style — showing the part's
  rhythm in regular note values with slash noteheads (stems, beams,
  dots, rests, triplet brackets), beamed by the meter's beat grouping.
  Where a change cuts a pulse, the shortened note and the following rest
  are drawn as played;
- a **pitch staff** with clef, key signature and time signature (C and ¢
  for common and cut time), showing the voicing as stacked noteheads,
  spelled by the key, *only at change points*.

**Now and next.** One long strip of bars across a landscape screen —
never a zoomed-in four bars per line. A playhead moves left to right;
upcoming bars are already written ahead of it, so the musician reads
ahead exactly as with a printed lead sheet. The strip advances without
ever leaving the musician short of bars ahead: at least half a strip is
always in view ahead of the playhead. Two ways to advance are to be
tried and compared:

- *Rewrite in place* — when the playhead passes the middle, the half
  already played is rewritten with the bars after the right half; the
  playhead wraps to the left edge like reading a new line. Nothing on
  screen ever moves.
- *Half shift* — when the playhead passes the middle, the strip shifts
  left by half its length in one quick, short slide; bars stay in order.

**Key changes** are shown as in standard notation: a double barline with
the new key signature at the bar where the change takes effect, and the
cautionary key signature at the end of the bar before. The triggers panel
lists the change as well.

**Triggers panel.** A side panel listing triggers as they approach,
fire, and pass.

**Key wheel.** One large, readable wheel for the song, showing the
current section's key and following the lead part's chords:

- Roman numerals inside the wedges, chord names just outside;
- wedges ordered by the diatonic cycle of fifths (I – IV – vii° – iii –
  vi – ii – V), so common progressions read as steady motion around the
  ring;
- in minor keys, a degree with two common chords (v and V, VII and
  vii°) keeps one wedge, with the alternative as a second, smaller
  label; the one actually playing lights up;
- wedges subtly coloured by function — tonic, subdominant, dominant —
  with an option to turn colour off;
- the current chord's wedge brightened and outlined, the others dimmed.
  The wheel shows only what is playing now; what comes next is read
  from the staff;
- a chord outside the key is shown in the centre, by name, rather than
  forced onto a wedge.

**Scaling.** How many bars fit across, and how many parts fit down,
depend on display size and resolution, not on a fixed limit. A simple song (chords and bass) is
drawn large and well spaced; a dense one is drawn smaller, down to a
legible minimum. *Example:* a 27" 1440p monitor today, a larger,
higher-resolution one later. Spreading parts over several windows or
monitors is a later step.

## Authoring view

Part of the program rather than a separate tool. The musician builds a
song: picks a progression for each section, sets key (from a list),
meter, tempo and repeats, lays rhythm patterns over bars for each part, chooses voicings,
adds phrases and triggers, marks the lead part, and assigns each part to
an instrument.

**Song file.** The song is saved as plain text in a notation of our own —
simple, clear to the musician reading or editing it by hand, and
unambiguous to the program.

## A building block

The Live Lead Sheet is the first link in a chain, not only an end in
itself. What it knows — key, meter, sections, tempo, bar and beat, the
chord now and the chords coming — is published for other programs to
follow, alongside the MIDI it sends. Its own screen follows the same
published information.

Later links could be:

- a melody generator that follows the chord changes;
- a transform, *e.g.* a transposition that keeps to the key;
- Strudel or another generative or feedback-based processor;
- a recorder that captures a melody played over the song and merges it
  into the song as a new part;
- other musicians over the internet, playing the same song together.

None of these is built in V1, but the design must allow them. MIDI
alone cannot carry this: it has notes, but no chord names, key, section
or knowledge of what comes next. The MIDI router stays at the MIDI
level; the chain runs at this musical level.

## Roles

**Leader (first).** Plays a song. It is the defining source of tempo,
beat and bar, sends Start/Stop and MIDI clock to external sequencers,
and sends each part's notes to its instrument. It also sends a **beat
clock as a note stream** for devices that follow note pulses better than
MIDI clock.

A **tempo slider** lets the musician slow down or speed up the written
tempo while practising.

**Listener (later).** Reads incoming MIDI from sequencers, recognises
chords, learns loops, and shows the bars it expects next. Each part shows
whether it is *learning*, *locked* onto a loop, or *drifting* from it.
The musician can mark the downbeat (tap "this is 1") and declare loop
length when detection is wrong.

**Recorder (later).** Captures a progression played or observed and adds
it to the library, so it can be led next time.

## Keys and scales

All twelve major and minor keys. Minor keys cover the standard minor
forms (natural, harmonic, melodic), so their usual chords — *e.g.* both
v and V, VII and vii° — belong on the wheel.

Alternative tunings and temperaments (Pythagorean, Tartini–Vallotti,
microtonal scales) are not in V1, but the design must leave a place for
them. Twelve-note temperaments change only the pitch of each note, not
the notation; the retuning belongs to the receiving instrument (*e.g.*
the Disting NT's microtonal tuning files). Scales with more than twelve
notes per octave would also need new accidentals and per-note pitch
control — a later problem.

## Meters

3/4, 4/4, 6/8, 5/4, 7/8 and similar, each with a stated beat grouping
(5/4 as 3+2 or 2+3; 7/8 as 2+2+3, etc.), and common or cut time.

## Versions

**V1** targets a chordal song: a chord part with its own slash rhythm
and a bass accompaniment, in diatonic harmony, on one screen.

Later versions bring denser songs, more parts, richer harmony, and more
display space. Choices made for V1 — especially in the renderer — must
leave room for that growth rather than build in V1's limits.

## Example setup

The musician improvises on a keyboard (NiftyKEYZ, KeyStep Pro). The Live
Lead Sheet leads a looping 12-bar blues in A. The chord part, marked
no root, goes to Disting NT poly players. The bass part sends the roots
through the FH-2 to a mono modular voice. MIDI clock and Start go through
the MIDI router to a Korg SQ-1.

Later, in listener mode, it reads a Minilogue's sequenced 4-voice chord
progression and names the chords that were forgotten.

## Out of scope

- Full notation engraving (melody, ties across systems, multi-voice
  staves) — use notation software.
- Piano rolls.
- MIDI routing, merging, filtering — the MIDI router's job. The Live
  Lead Sheet is just another port to the router.
- Splitting a chord over several mono voices — the receiving device's
  job (*e.g.* the FH-2).
- Generating music.

## Starter library

**Progressions:** 12-bar blues (basic, quick-change, jazz blues with
ii–V turnarounds), minor blues, 8-bar blues, ii–V–I in major and minor,
I–vi–ii–V, I–V–vi–IV. Modal vamps and rhythm changes come later.

**Rhythm patterns:** whole, half and quarter pulses, eighths,
quarter-note triplets, Charleston, push (an eighth before the beat), a
basic 6/8 pulse, a bossa-style comp.
