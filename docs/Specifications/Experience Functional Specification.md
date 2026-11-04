# Live Lead Sheet — Experience Functional Specification

Revision 10 — 4 Nov 2026

Builds on the Concept Brief, Revision 10, and Architecture D-10. Covers
**V1: the leader** — a chordal song with its own slash rhythm and a bass
accompaniment, with a control track for the beat pulse and phrase cues,
on one screen. Requirements are in the musician's terms; nothing here chooses a
language, library or internal design.

## 1. Play screen

**EFS-1.1** The play screen shows at once: the song overview, the
counter, the tempo, the key wheel, the cues panel, and the voice
tracks.

**EFS-1.2 Song overview.** A strip across the top shows every section in
order, by name, with its repeat count. The current section and the
position within it are marked.

**EFS-1.3 Counter.** Large and plain: `Repeat: 3 of 4   Bar: 2   Count: 1`.
Count follows the time signature, one count per unit of its lower number:
4/4 counts 1–4, 3/4 counts 1–3, 6/8 counts 1–6, 5/8 counts 1–5, and
2/2 (cut time) counts 1–2. In a count-in section the counter says so:
`Count-in   Bar: 1   Count: 3`.

**EFS-1.3a Playhead.** The playhead glides smoothly through the bar and
pulses brightly on each count.

**EFS-1.4 Tempo.** The current tempo in bpm, and the slider setting when
it differs from the written tempo.

**EFS-1.5 Voice tracks.** Each voice track has its own contained area,
in the order set in the song. Each area has, top to bottom:

- **chord symbols**, placed at the exact position of each change, even
  mid-beat, spelled by the key;
- a **rhythm line** — a one-line staff with slash noteheads in regular
  note values: stems, flags, beams grouped by the meter's beat grouping,
  dots, rests, triplet brackets, and ties where a value crosses a beat or
  barline. A pulse cut short by a chord change is drawn shortened,
  followed by a rest;
- a **pitch staff** with clef, key signature and time signature (C and ¢
  for common and cut time), showing each voicing as stacked noteheads,
  only at change points.

Chord symbols and voicings have one size throughout the song, the
smallest that stays legible; bars never widen to make room, and nothing
shrinks to fit. Chord symbols stand on one row, each starting on its
pulse's stem. Where a bar's symbols would touch, that bar alternates them
between a lower and an upper row, continuing into the next bar if it is
crowded too.

The pitch staff is shown by default. A plainly visible control beside
each track's pitch staff hides it and shows it again, at any time, also
while playing. Hidden, the track keeps its chord symbols and rhythm line,
and the space goes to other tracks (EFS-1.12).

**EFS-1.5a Chord naming.** Every chord name, on the strip, the wheel and
in the authoring view, follows the standard convention:

- the root is a capital letter, with its accidental: C, F♯, B♭;
- minor is a lower-case **m**: Am, Am7, F♯m7♭5;
- major seventh and its extensions use upper-case **M**: CM7, CM9 — so
  C7 (dominant) and CM7 (major seventh) can never be confused;
- a plain major triad is the letter alone: C;
- diminished °, half-diminished ø, augmented +: B°, Bø7, C+;
- an inversion or other bass note after a slash: C/E.

Roman numerals follow the same rule: upper case for major (I, IV, V7),
lower case for minor (ii, vi), with ° ø + as above (vii°).

**EFS-1.5b Clefs.** Each voice track's clef is set in the song and never
changes while playing: treble for a harmony track, bass for a bass
track, or a grand staff (treble and bass braced) for a harmony track
whose voicings run well
below middle C. Notes outside the staff use ledger lines or an 8va/8vb
line. The rhythm line uses the neutral (percussion) clef. Other clefs and
mixed clefs within a track are not needed.

**EFS-1.6 The strip.** Bars run left to right in one long strip, in the
order they will actually be played — repeats are written out, not shown
as repeat signs. Section starts have a double barline and the section
name.

**EFS-1.7 Reading ahead.** At least half the strip is always ahead of the
playhead.

**EFS-1.8 Advancing.** Both advance methods from the Concept Brief are
available, selectable while playing, so they can be compared:
*rewrite in place* and *half shift*.

**EFS-1.9 Key changes.** As in standard notation: a key change shows a
double barline and the new key signature at the bar where it takes
effect, and the cautionary key signature at the end of the bar before.
The cues panel lists it (EFS-1.11).

**EFS-1.10 Key wheel.** One large wheel for the song:

- seven wedges in the diatonic cycle of fifths (I – IV – vii° – iii –
  vi – ii – V), Roman numerals inside, chord names just outside, spelled
  by the key;
- in minor keys, a degree with two common chords keeps one wedge, with
  the alternative as a second, smaller label;
- the lead track's current chord: its wedge brightened and outlined, the
  other wedges dimmed, with colour on or off. The wheel shows only what is
  playing now; what comes next is read from the staff;
- a chord outside the key shown in the centre, by name;
- subtle colour by function (tonic, subdominant, dominant), with colour
  switchable off.

**EFS-1.11 Cues panel.** A side panel listing what is coming: each
phrase cue, key change and tempo change in view, with its bar and count.
Each entry is marked when it fires and leaves the list once passed.

**EFS-1.12 Scaling.** Bars across and tracks down are set by screen size
and resolution, down to a legible minimum. If the voice tracks cannot
all fit at that minimum, the lead track is always shown and the screen
says plainly which tracks are hidden — never silently.

**EFS-1.13 Look.** Dark theme with soft contrast throughout, including
every control. Chord symbols, counter and wheel are readable at arm's
length.

**EFS-1.14 Warnings region.** A small region of the play screen lists
warnings as they happen (see section 8). It never covers the music.

## 2. Transport and timing

**EFS-2.1 Play and stop.** Play from the start of the song (its
count-in section included), from the start of the current section, or
from the start of any chosen section (no count-in; see EFS-2.8 for
practice).
A start control on the play screen chooses between two modes: start from
the beginning of the song, or start at the play position (the start of
the current or chosen section). Where to start is the player's choice,
set on the play screen, whichever clock source is in use (EFS-2.9).
Stop sends MIDI Stop and silences every track
(All Notes Off) on every output port in use.

**EFS-2.2 Tempo.** Each section has a written tempo, or carries on the
previous section's tempo.

**EFS-2.3 Tempo slider.** For practice, a slider scales the written
tempo, as a percentage, so tempo changes between sections keep their
proportions. Range 50–150 %.

**EFS-2.4 Steady time.** Tempo is even, with no drift over a long
performance.

**EFS-2.5 MIDI clock.** MIDI clock (24 per quarter note) is sent on
exactly one output port, the clock output, from Play to Stop; the
program is one clock source. Connecting the clock output to each device
that follows clock is the studio's setup (a MIDI router, `aconnect`, or
other), not the program's. External sequencers are started, stopped and
reset by phrase cues (EFS-4).

**EFS-2.6 Beat pulse.** A control track that sends one note per count,
for clock dividers and devices that follow notes better than MIDI clock.
Port, channel and note number are set by the musician. Count 1 of each
bar is sent at an accent velocity, every other count at a normal
velocity — 127 and 64 unless the track sets other values — so a
MIDI-to-CV converter can trigger a percussive voice and set its VCA
level from the same stream, and the bar's first beat is always
distinguishable. The beat pulse does not depend on MIDI transport, so a
gate made from it (e.g. on an FH-2 output) keeps clock dividers running
while sequencers are stopped.

**EFS-2.7 Count-in section.** A song may begin with a count-in section of
one or two bars (two help at fast tempos): the voice tracks rest, the
beat pulse plays, and the counter shows the count-in. A phrase cue on
the music's first bar starts the external sequencers, so they do not
play the count-in and all start together on that downbeat.

**EFS-2.8 Loop for practice.** Loop the current section until released,
regardless of its written repeats. A *count-in first* control on the
play screen (on or off, kept until changed) plays the song's count-in
section once before the first pass; the section then loops without a
count-in until stopped. Each Play honours the setting. If the song has
no count-in section, the control is unavailable and says so.

**EFS-2.9 Following an external MIDI clock — later.** Not in V1, but
nothing in V1 may block it. A plainly visible clock-source control
chooses *internal* (the program sends MIDI clock, EFS-2.5) or *external*
(it follows the MIDI clock arriving on one chosen input, e.g. from the
FH-2). The clock source can be changed only while stopped: one source is
in charge of transport at a time. While following:

- The clock owner's Start, Stop, Continue and Song Position Pointer
  control the transport. The screen's Play, Stop and play-from controls
  are disabled; the screen follows what arrives.
- A received Song Position Pointer sets the play position. On Start or
  Continue from the clock owner, playing starts where the start control
  says (EFS-2.1). The clock owner's position 0 is the music's first bar:
  the count-in section is not played.
- Tempo is the clock owner's. The tempo shown (EFS-1.4) is the tempo
  measured from the incoming clock; the tempo slider is disabled (greyed
  out or hidden, settled with the screen layout); written tempo changes
  have no effect.
- If no clock arrives for 1 s while playing, the program stops, silences
  its tracks, and says so, e.g.
  `No clock from FH-2. Stopped. Press Play on FH-2 to restart.`
  The 1 s is a starting value, to be tested; it may need to be shorter.
- The incoming clock is not passed on to the program's outputs; that is
  a MIDI router's job.
- Following the clock and the score comes first; the screen may lag and
  catch up.

## 3. Tracks and MIDI output

**EFS-3.1 Output ports.** The program offers output ports named by the
musician for their purpose (e.g. `Clock`, `Harmony`, `SQ-1`). Each voice
track sends on one output port and channel; each phrase cue names its
own (EFS-4.3). Exactly one port is the clock output (EFS-2.5).

**EFS-3.2 Harmony track.** On each pulse of its rhythm, sends the current
chord's voicing. When marked *no root*, the root is left out.

**EFS-3.3 Bass track.** On each pulse of its rhythm, sends the current
chord's root, in the octave set for the track.

**EFS-3.4 Chord change between pulses.** The old chord ends exactly at
the change; the next pulse sounds the new chord. Two chords never sound
at once in one track.

**EFS-3.5 Note length.** Each voice track has a gate length, as a
percentage of each pulse's written value; 50 % unless set otherwise.

**EFS-3.6 Velocity.** One velocity per voice track in V1.

**EFS-3.7 Lead track.** Exactly one voice track is marked lead; its
chords drive the key wheel.

## 4. Control tracks and phrase cues

**EFS-4.1** A phrase cue is a named marker at a bar and count in a
section, on a control track or on a voice track.

**EFS-4.2** A cue is shown above the strip at its position and listed in
the cues panel (EFS-1.11).

**EFS-4.3** In V1 a cue sends one plain MIDI message: a note (number,
velocity and length — a short note is a trigger, a held one a gate), a
CC, a program change, or — on a control track only — Start, Stop or
Continue. A cue on a control track names its output port and channel
(a port only, for Start, Stop and Continue, which have no channel). A
cue on a voice track goes to that track's own port and channel.

**EFS-4.4 Passes.** A cue in a repeated section can be limited to chosen
passes, written as in standard notation ("1st time only").

**EFS-4.5 Playing from a section.** When playing starts at a section
(EFS-2.1, EFS-2.8), the program first sends the state in effect there:
the last program change and CC value of each cue target, and any note
held as a gate. External sequencers are put in step by reset cues at
their phrase boundaries; Song Position Pointer is not used, because many
devices ignore it. A section start that falls inside an external
sequencer's phrase is warned about in the authoring view.

**EFS-4.6 External phrases — later.** An external sequencer's phrase
length is set as steps × step value (e.g. 8 steps of eighths), so a
named action ("SQ-1: pattern 3, 2 runs from bar 5") can be turned into
cues by a device profile. In V1, cues are placed by hand.

**EFS-4.7 Pulse bursts — later.** A cue can start a burst of fast pulses
— notes on a chosen port and channel, at a set rate (e.g. 64ths) for a
set length — to clock a sequencer far above the song's tempo, as an
oscillator, or to fire fast voices. Timing Clock stays at the song's
tempo (EFS-2.5).

## 5. Authoring view

**EFS-5.1 Songs.** Create, open and save songs.

**EFS-5.2 Sections.** Add, name, reorder and remove sections, and mark
the first as a count-in section. For each:
repeat count, key (twelve roots, major or minor, chosen from a list),
meter with its beat grouping, and tempo (written, or carry on).

**EFS-5.2a Minor keys.** A minor key uses the natural-minor key
signature, as in standard notation. Chords that use the raised sixth or
seventh degree (*e.g.* E7 in A minor) are written with accidentals,
named by their actual notes, and shown on the wheel as the degree's
alternative chord. A section never has to choose a minor form.

**EFS-5.3 Chords.** Start a section from a library progression, then
change any chord, or where it falls within the bar.

**EFS-5.4 Rhythm.** Assign library rhythm patterns to bar ranges, track
by track. A pattern repeats or is cut to fill its range.

**EFS-5.5 Voicings.** Each chord starts with a plain voicing — close
position, root position, in the track's octave — which the musician can
change note by note. Nothing else is chosen automatically.

**EFS-5.6 Tracks.** Add, name and remove voice tracks and control
tracks. For a voice track: its kind (harmony or bass), output port and
channel, octave, lead marker, and *no root*. For a beat pulse track: its
output port, channel, note, and velocities.

**EFS-5.7 Phrase cues.** Place, name, move and remove cues, and set each
cue's message, target and passes.

**EFS-5.8 See it as it plays.** Edits show on the play strip straight
away.

**EFS-5.9 Library.** Browse library progressions and rhythm patterns,
and save a progression or rhythm from the song into the library.

**EFS-5.10 Navigation marks — later.** Not in V1, but the song structure
must leave room for the standard navigation marks: D.C., D.S., segno,
Coda and To Coda, Fine, and first and second endings. When they arrive,
the strip still shows bars in the order they will actually be played.

## 6. Song file and library files

**EFS-6.1** A song is one plain-text file in our own notation: simple,
clear to the musician reading or editing it by hand, unambiguous to the
program.

**EFS-6.2** What the authoring view saves, it loads back exactly. A
hand-edited file loads as written.

**EFS-6.3** A file with a mistake does not load partly or quietly. The
message names the file, the line, what is wrong, and what to type
instead — *e.g.* `blues.song line 14: "IV9b" is not a chord. Try IV9 or IV7b9.`

**EFS-6.4 MusicXML.** A song can be saved as a MusicXML file, so it
opens in notation software: chord symbols, key and time signatures,
sections, and each voice track's notes as actually played (voicings on their
rhythms).

**EFS-6.5** Library progressions and rhythm patterns are plain-text files
in the same notation.

## 7. Starter library

**EFS-7.1 Progressions.** 12-bar blues (basic, quick-change, jazz blues
with ii–V turnarounds), minor blues, 8-bar blues, ii–V–I in major and
minor, I–vi–ii–V, I–V–vi–IV.

**EFS-7.2 Rhythm patterns.** Whole, half and quarter pulses, eighths,
quarter-note triplets, Charleston, push (an eighth before the beat), a
basic 6/8 pulse, a bossa-style comp.

## 8. Messages and failures

**EFS-8.1 Keep playing.** The program keeps playing through failures. An
instrument or port that vanishes does not stop the song: the leader
keeps sending, and a message sent to a missing receiver simply goes
nowhere. Senders and receivers stay independent.

**EFS-8.2 Nothing silent.** Every failure appears in the warnings region
(EFS-1.14): what happened and exactly what to do, in a few plain words,
readable mid-session — *e.g.* `FH-2 gone. Plug it back in; it rejoins on
the next bar.`

**EFS-8.3 No sound alerts.** The program never signals a failure with a
sound.

**EFS-8.4 A second clock.** A follower counts every Timing Clock that
reaches it, so a second clock source reaching a port the program sends
clock to disturbs the followers on that port. On an FH-2, while its
transport ran, the tempo display and the configured clock output both
showed the two clocks' rates added together, unevenly (MIDI Router
project, FH-2 Observations, 1 Nov 2026). A second clock can appear
without the musician choosing it: a device that sends its own clock is
powered on, or another program connects a clock source to the port.
When the program detects another clock source reaching, or connected
to, a port it sends clock to, it keeps playing (EFS-8.1) and names the
source and the port, *e.g.*
`Second clock into FH-2 from NiftyKEYZ. Unplug NiftyKEYZ or stop its clock.`
Which cases the program can detect, and how, is established by testing
before this is built.

## 9. Remote control

**EFS-9.1 Remote control from a MIDI controller.** Every control on the
play screen can also be worked from the buttons, knobs and sliders of a
MIDI controller (e.g. a Launch Control XL 3), so the musician can run the
player from a distance without reaching for the mouse. A control added
to the play screen is therefore always one a MIDI controller can work.
In V1, built after the mouse controls work; the mouse controls never
wait for it. How controls are assigned to the controller's buttons,
knobs and sliders is settled before it is built.

## 10. Not in V1

Listener and recorder roles; colour-phrase tracks; device profiles and
external phrases (EFS-4.6); pulse bursts (EFS-4.7); alternative tunings;
modes and modal vamps; navigation marks (D.C., D.S., Coda…); SATB voices
(one line per voice, one channel per voice); other programs following
the song (see the Concept Brief, "A building block"); several windows or
monitors; following an external MIDI clock (EFS-2.9).
