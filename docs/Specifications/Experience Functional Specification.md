# Live Lead Sheet — Experience Functional Specification

Revision 12 — 5 Nov 2026 (song and studio configuration are separate files; track types; marks; cues in rhythm notation; errors shown, not fatal)

Builds on the Concept Brief, Revision 12, Architecture D-10 and D-11, and
Song Notation, Revision 6. Covers
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

- **chord symbols**, shown at the rhythm element where each chord starts,
  even an element in mid-beat, spelled by the key;
- a **rhythm line** — a one-line staff with slash noteheads in regular
  note values: stems, flags, beams grouped by the meter's beat grouping,
  dots, rests, triplet brackets, and ties where a value crosses a beat or
  barline. The line always shows the rhythm as written; a chord change
  never alters it (EFS-3.4);
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
below middle C. The composer composes in the clef, so most notes sit on
the staff; notes outside it use ledger lines or an 8va/8vb line. The
rhythm line uses the neutral (percussion) clef. The alto and tenor clefs
are for parts a musician plays (later, EFS-3.8); mixed clefs within a
track are not needed.

**EFS-1.5c Marks.** Accent, strong accent, staccato, legato and fermata
marks (EFS-3.5, EFS-3.6) are drawn as in standard notation on the
element that carries them, in the rhythm line and on the pitch staff.

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
previous section's tempo. The first section's tempo is the song's initial
tempo; a tempo or time-signature change happens at a section boundary.
Later: tempo marks inside a section, a step or a ramp between two
points in the song (an accelerando, or a ritardando at the end), written in
rhythm notation (Song Notation, section 5).

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
Port, channel, note number and velocities are set by the musician in the
studio configuration (EFS-6.6). The first beat of each bar is sent at
an accent velocity, every other beat at a normal velocity — 127 and 64
in the usual setup — in any meter, so 4/4 and 6/8 are treated alike. A
MIDI-to-CV converter can trigger a percussive voice and set its VCA
level from the same stream, and the bar's first beat is always
distinguishable. The beat pulse does not depend on MIDI transport, so a
gate made from it (e.g. on an FH-2 output) keeps clock dividers running
while sequencers are stopped. It is not a drum part; a second use is a
metronome voiced in the modular.

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
musician for their purpose (e.g. `Clock`, `Harmony`, `SQ-1`), declared in
the studio configuration (EFS-6.6), never in the song. Each voice track
sends on one output port and channel, set there; each phrase cue goes to
a named destination, set there too (EFS-4.3). Exactly one port is the
clock output (EFS-2.5).

**EFS-3.2 Poly voice track (harmony).** A track that can play several
notes at once. Each sounding element of its rhythm carries a chord, written
in the song by name, or notes. A chord sends its voicing: the plain voicing
of EFS-5.5 unless the musician changed it. A voicing without the root
(where the bass plays the root) is such a change, written note by note;
the editor can write it for the whole line. There is no track-wide *no
root* setting.

**EFS-3.3 Mono voice track (bass).** A track that plays one note at a
time. Each sounding element of its rhythm carries the note it plays,
written in the song. A bass line is free: it may play the chord's root,
another chord note, or any other note. The song does not accept
overlapping notes on a mono track, in a hand-written file as well as in
the editor. When the musician starts a bass line from a progression
(EFS-5.4), the roots of the chords are offered as its notes, in the
track's home octave, and any note can be changed.

**EFS-3.4 Voice tracks play what is written.** Nothing is looked up when
the song plays. A sounding element sends the notes written on it: note on
at the element, note off when its written duration ends (EFS-3.5), rests
included. A chord in the harmony track is simply what its elements carry;
a chord change does not shorten or end a sounding element. Another track's
notes need not match the harmony track's chords. A track is silent
wherever its rhythm has a rest, and the elements of one track do not
overlap, so two sets of notes never sound at once in one track.

**EFS-3.5 Note length.** A note sounds for its written duration: the note
off falls where the written duration ends, less at most a short fixed gap
if a driven device needs one to retrigger (a gate into an envelope). The
gap belongs to the device, so it is set per track in the studio
configuration (EFS-6.6); whether it is needed at all is settled by testing
(Song Notation, Q11). There is no gate-length percentage. Two marks
change the length of a single note, as in standard notation: *staccato*
sounds it for half its written duration, and *legato* joins it to the
next note, with no gap between them. (A cue's note carries its own
length, EFS-4.3.)

**EFS-3.6 Velocity and accents.** The velocity of a note is a device
setting, not a song setting. In the studio configuration each voice track
has a normal velocity and, for each accent mark the song uses, an accent
velocity (*accent* and *strong accent*). In the song, an accent mark on a
sounding element selects the level. Rhythm patterns carry accent marks
(a backbeat on counts 2 and 4), and apply copies them. Pressure, pitch
bend and other controllers are later (EFS-3.8).

**EFS-3.7 Lead track.** Exactly one voice track is marked lead; its
chords drive the key wheel and the chord symbols.

**EFS-3.8 Track types.** Each track has a type. In V1: poly voice and mono
voice (EFS-3.2, 3.3), beat pulse (EFS-2.6), clock and transport, and cue
control (EFS-4). The clock and transport track has a port and no channel,
because Timing Clock, Start, Continue and Stop carry none; each song has
exactly one. Later: a tempo control track (EFS-2.2), a musician voice
(a part composed for a person to read, in the clef of that instrument,
sending no MIDI; a fermata is drawn, never obeyed by the clock), and an
automation control track (continuous controller curves, such as a filter
sweep or a crescendo).

**EFS-3.9 Pitch.** A song's pitches are staff pitches in the track's clef
(C4 is middle C), not MIDI note numbers. A device that places its notes
an octave or more away from the staff is corrected in the studio
configuration with an octave offset (EFS-6.6). For a transposing instrument
(later) the song holds the sounding pitch, and the staff shows the written
pitch.

## 4. Control tracks and phrase cues

**EFS-4.1** A phrase cue is a named marker in a section, on a cue
control track or aimed at a voice track. It is placed by writing a rhythm
line, like any other track: the cue sits on the element where it fires, so
the musician counts note values and not bar numbers across repeats.

**EFS-4.2** A cue is shown above the strip at its position and listed in
the cues panel (EFS-1.11).

**EFS-4.3** In V1 a cue sends one plain MIDI message: a note (number,
velocity and length — a short note is a trigger, a held one a gate), a
CC, a program change, or — on a control track only — Start, Stop or
Continue. A cue names a destination, which the studio configuration maps
to an output port and channel (a port only, for Start, Stop and Continue,
which have no channel). A cue aimed at a voice track goes to that track's
own port and channel. For a note cue, the written duration of its element
is the note's length.

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
change any chord, or the rhythm element it starts on.

**EFS-5.4 Rhythm.** Assign library rhythm patterns to bar ranges, track
by track. A pattern repeats or is cut to fill its range. Applying a
rhythm puts the range's chords on the rhythm's elements (with the
rhythm's accent marks), and a bass line can be started from the chords'
roots; the musician can move or change any of them. The song keeps the
result, not a link to the library entry.

**EFS-5.5 Voicings.** Each chord starts with a plain voicing — close
position, root position, with the root in the track's home octave — which
the musician can change note by note, or write without the root.
Nothing else is chosen automatically.

**EFS-5.6 Tracks.** Add, name and remove tracks. For a voice track: its
type (poly or mono), clef, lead marker and home octave. Output port,
channel and the other device settings are not part of the song: they are
set in the studio configuration (EFS-6.6), in the same authoring view.

**EFS-5.7 Phrase cues.** Define cues (message and destination) and place,
move and remove them on the rhythm line of a cue control track; set each
cue's passes.

**EFS-5.11 Text pane.** The authoring view includes a plain text pane for
the song file, the studio configuration and the libraries, so a file with
errors can be fixed, saved and reloaded without leaving the program
(EFS-6.3).

**EFS-5.8 See it as it plays.** Edits show on the play strip straight
away.

**EFS-5.9 Library.** Browse library progressions and rhythm patterns,
and save a progression or rhythm from the song into the library.

**EFS-5.10 Navigation marks — later.** Not in V1, but the song structure
must leave room for the standard navigation marks: D.C., D.S., segno,
Coda and To Coda, Fine, and first and second endings. When they arrive,
the strip still shows bars in the order they will actually be played.

## 6. Song file and library files

**EFS-6.1** A song is one plain-text file (`.song`) in our own notation:
simple, clear to the musician reading or editing it by hand, unambiguous
to the program. It holds everything the song plays and refers to no
library entry, so a song stands alone when the libraries change. It names
no port, channel or device: it is the composition, and it can be played
through any studio configuration that covers its tracks.

**EFS-6.2** What the authoring view saves, it loads back exactly. A
hand-edited file loads as written.

**EFS-6.3** A file with a mistake does not load partly or quietly, and
nothing from it is applied or played. The program still starts and shows
the errors in the warnings region and the text pane (EFS-5.11); it checks
the file again each time it is saved. Each message names the file, the
line, what is wrong, and what to type instead — *e.g.* `blues.song line
14: "IV9b" is not a chord. Try IV9 or IV7b9.` If a song that is playing is
edited into error, the last good version keeps playing while the errors
are shown.

**EFS-6.4 MusicXML.** A song can be saved as a MusicXML file, so it
opens in notation software: chord symbols, key and time signatures,
sections, and each voice track's notes as actually played (voicings on their
rhythms).

**EFS-6.5** Library progressions and rhythm patterns are plain-text files
in the same notation: one file for each library (`.library`), its entries
in groups. Rhythms are grouped by meter, then by genre; progressions by
idiom, then by kind.

**EFS-6.6 Studio configuration.** The studio configuration is one
plain-text file (`.studio`) in the same notation. It declares the output
ports and, for each track of a song by its name: port, channel (none for
the clock and transport track), octave offset, velocity levels
(EFS-3.6), and device timing — a retrigger gap and a settle delay for a
device that needs time (EFS-3.5) — and it maps each cue destination to a
port and channel. Quantities are written with their units (`2 ms`, `-1
octaves`). One configuration serves many songs; a track of the song with
no entry stops the song loading, and the message names the track. Errors
are handled as in EFS-6.3.

## 7. Starter library

**EFS-7.1 Progressions.** 12-bar blues (basic, quick-change, jazz blues
with ii–V turnarounds), minor blues, 8-bar blues, ii–V–I in major and
minor, I–vi–ii–V, I–V–vi–IV. Grouped as Blues, Jazz, and Pop and rock;
the musician adds groups, for example from classical composers.

**EFS-7.2 Rhythm patterns.** Whole, half and quarter pulses, eighths,
quarter-note triplets, Charleston, anticipation (an eighth before the beat), a
basic 6/8 pulse, a bossa-style comp, a backbeat with accents on counts 2
and 4. Grouped by meter, then Pulses, Swing, Rock and Latin.

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

Listener and recorder roles; colour-phrase tracks; musician voices and
automation tracks (EFS-3.8); tempo marks and ramps inside a section
(EFS-2.2); lines of different lengths in a section (polymeter); soft
notes, pressure and pitch bend; device profiles and
external phrases (EFS-4.6); pulse bursts (EFS-4.7); alternative tunings;
modes and modal vamps; navigation marks (D.C., D.S., Coda…); SATB voices
(one line per voice, one channel per voice); other programs following
the song (see the Concept Brief, "A building block"); several windows or
monitors; following an external MIDI clock (EFS-2.9).
