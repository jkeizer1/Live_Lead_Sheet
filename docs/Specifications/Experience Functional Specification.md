# Live Lead Sheet — Experience Functional Specification

Revision 5 — 28 Oct 2026

Builds on the Concept Brief, Revision 7. Covers **V1: the leader** — a
chordal song with its own slash rhythm and a bass accompaniment, on one
screen. Requirements are in the musician's terms; nothing here chooses a
language, library or internal design.

## 1. Play screen

**EFS-1.1** The play screen shows at once: the song overview, the
counter, the tempo, the key wheel, the triggers panel, and the parts.

**EFS-1.2 Song overview.** A strip across the top shows every section in
order, by name, with its repeat count. The current section and the
position within it are marked.

**EFS-1.3 Counter.** Large and plain: `Repeat: 3 of 4   Bar: 2   Count: 1`.
Count follows the time signature, one count per unit of its lower number:
4/4 counts 1–4, 3/4 counts 1–3, 6/8 counts 1–6, 5/8 counts 1–5, and
2/2 (cut time) counts 1–2.

**EFS-1.3a Playhead.** The playhead glides smoothly through the bar and
pulses brightly on each count.

**EFS-1.4 Tempo.** The current tempo in bpm, and the slider setting when
it differs from the written tempo.

**EFS-1.5 Parts.** Each part has its own contained area, in the order
set in the song. Each area has, top to bottom:

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

**EFS-1.5b Clefs.** Each part's clef is set in the song and never changes
while playing: treble for a chord part, bass for a bass part, or a grand
staff (treble and bass braced) for a chord part whose voicings run well
below middle C. Notes outside the staff use ledger lines or an 8va/8vb
line. The rhythm line uses the neutral (percussion) clef. Other clefs and
mixed clefs within a part are not needed.

**EFS-1.6 The strip.** Bars run left to right in one long strip, in the
order they will actually be played — repeats are written out, not shown
as repeat signs. Section starts have a double barline and the section
name.

**EFS-1.7 Reading ahead.** At least half the strip is always ahead of the
playhead.

**EFS-1.8 Advancing.** Both advance methods from the Concept Brief are
available, selectable while playing, so they can be compared:
*rewrite in place* and *half shift*.

**EFS-1.9 Key changes.** A key change shows a double barline and the new
key signature where it takes effect, and the cautionary key signature at
the end of the bar before. Once it is in view, the wheel shows
"Next: E♭" faintly, and the triggers panel lists it.

**EFS-1.10 Key wheel.** One large wheel for the song:

- seven wedges in the diatonic cycle of fifths (I – IV – vii° – iii –
  vi – ii – V), Roman numerals inside, chord names just outside, spelled
  by the key;
- in minor keys, a degree with two common chords keeps one wedge, with
  the alternative as a second, smaller label;
- the lead part's current chord filled, its next chord outlined;
- a chord outside the key shown in the centre, by name;
- subtle colour by function (tonic, subdominant, dominant), with colour
  switchable off.

**EFS-1.11 Triggers panel.** A side panel listing what is coming: each
trigger, key change and tempo change in view, with its bar and count.
Each entry is marked when it fires and leaves the list once passed.

**EFS-1.12 Scaling.** Bars across and parts down are set by screen size
and resolution, down to a legible minimum. If the parts cannot all fit at
that minimum, the lead part is always shown and the screen says plainly
which parts are hidden — never silently.

**EFS-1.13 Look.** Dark theme with soft contrast throughout, including
every control. Chord symbols, counter and wheel are readable at arm's
length.

**EFS-1.14 Warnings region.** A small region of the play screen lists
warnings as they happen (see section 8). It never covers the music.

## 2. Transport and timing

**EFS-2.1 Play and stop.** Play from the start of the song, from the
start of the current section, or from the start of any chosen section. Stop sends MIDI Stop and silences every part
(All Notes Off) on every port in use.

**EFS-2.2 Tempo.** Each section has a written tempo, or carries on the
previous section's tempo.

**EFS-2.3 Tempo slider.** For practice, a slider scales the written
tempo, as a percentage, so tempo changes between sections keep their
proportions. Range 50–150 %.

**EFS-2.4 Steady time.** Tempo is even, with no drift over a long
performance.

**EFS-2.5 MIDI clock.** MIDI clock (24 per quarter note) and Start/Stop,
sent to each chosen output port.

**EFS-2.6 Beat clock as notes.** A note pulse, one per count, for devices
that follow notes better than MIDI clock. Port, channel and note number
are set by the musician. Count 1 of each bar is sent at full velocity,
every other count at half — 127 and 64 unless the song sets other
values — so a MIDI-to-CV converter can
trigger a percussive voice and set its VCA level from the same stream,
and the bar's first beat is always distinguishable.

**EFS-2.7 Count-in.** An optional count-in of one or two bars (two help
at fast tempos): clock and beat notes run, parts are silent, and the
counter counts down.

**EFS-2.8 Loop for practice.** Loop the current section until released,
regardless of its written repeats.

## 3. Parts and MIDI output

**EFS-3.1** Each part sends to one output port and channel.

**EFS-3.2 Chord part.** On each pulse of its rhythm, sends the current
chord's voicing. When marked *no root*, the root is left out.

**EFS-3.3 Bass part.** On each pulse of its rhythm, sends the current
chord's root, in the octave set for the part.

**EFS-3.4 Chord change between pulses.** The old chord ends exactly at
the change; the next pulse sounds the new chord. Two chords never sound
at once in one part.

**EFS-3.5 Note length.** Each part has a gate length, as a percentage of
each pulse's written value; 50 % unless set otherwise.

**EFS-3.6 Velocity.** One velocity per part in V1.

**EFS-3.7 Lead part.** Exactly one part is marked lead; its chords drive
the key wheel.

## 4. Triggers

**EFS-4.1** A trigger is a named marker at a bar and count in a section.

**EFS-4.2** A trigger is shown above the strip at its position and listed
in the triggers panel.

**EFS-4.3** In V1 a trigger sends one plain MIDI message to a chosen
port and channel: a note (number, velocity and length — many devices
respond to notes but not to CCs), a CC, or a program change. Device
profiles, which turn a named action into the right messages for a given
device, come later.

## 5. Authoring view

**EFS-5.1 Songs.** Create, open and save songs.

**EFS-5.2 Sections.** Add, name, reorder and remove sections. For each:
repeat count, key (twelve roots, major or minor, chosen from a list),
meter with its beat grouping, and tempo (written, or carry on).

**EFS-5.2a Minor keys.** A minor key uses the natural-minor key
signature, as in standard notation. Chords that use the raised sixth or
seventh degree (*e.g.* E7 in A minor) are written with accidentals,
named by their actual notes, and shown on the wheel as the degree's
alternative chord. A section never has to choose a minor form.

**EFS-5.3 Chords.** Start a section from a library progression, then
change any chord, or where it falls within the bar.

**EFS-5.4 Rhythm.** Assign library rhythm patterns to bar ranges, part
by part. A pattern repeats or is cut to fill its range.

**EFS-5.5 Voicings.** Each chord starts with a plain voicing — close
position, root position, in the part's octave — which the musician can
change note by note. Nothing else is chosen automatically.

**EFS-5.6 Parts.** Add, name and remove parts; set each part's kind
(chord or bass), port and channel, octave, lead marker, and *no root*.

**EFS-5.7 Triggers.** Place, name, move and remove triggers.

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
sections, and each part's notes as actually played (voicings on their
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

## 9. Not in V1

Listener and recorder roles; colour-phrase parts; device profiles;
alternative tunings; modes and modal vamps; navigation marks (D.C.,
D.S., Coda…); SATB voices (one line per voice, one channel per voice);
other programs following the song (see the Concept Brief, "A building
block"); several windows or monitors.
