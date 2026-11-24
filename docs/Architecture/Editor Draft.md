# Live Lead Sheet — Editor, first draft

Revision 2 — 23 Nov 2026

A first working editor and player screen, built as a fast sprint to be tried,
not a specification. It is the basis for writing the authoring view's
requirements properly (EFS section 5). What it does is listed in `README.md`
("The program"); this note says why it is shaped as it is, what it leaves out,
and the questions it raised.

## The idea in one paragraph

The song text stays the one source of truth. Every edit the screen makes —
applying a progression or a rhythm, moving a chord by a degree, transposing a
section, changing a repeat — writes new song text, which the song file's own
reader reads back before it is taken. A text that would not load is refused,
with the reader's message, and nothing changes. So the file the editor saves
is always one that plays (EFS-6.2, EFS-6.3), it stays readable and editable
by hand in any editor, and undo is a list of earlier texts.

## Composing at the level the musician is thinking

The musician works on one level of the song at a time. Five **modes**, in
the order of the song's hierarchy, each render their own view, open their
own library tab and say where an edit goes. Each button names what it is for
and carries its key; the header always says what an edit changes now
("Editing: Bass line (coupled), "Verse" bar 2, note 3").

| Key | Mode | Works on | View | Library |
|---|---|---|---|---|
| F1 | Arrange Sections | the composition: sections in order, repeats, each entry's key and tempo, new sections | the composition as a list | Progressions (N: a new section) |
| F2 | Compose Sections | one section: its key and tempo, its lines' forms (coupled or sampled) | the section's lines as lanes | Progressions, onto the selected line as in F5 (to revisit) |
| F3 | Edit Lines | the music on a line, note by note | the staff | Rhythms |
| F4 | Song Harmony | the lead line's chords | every entry's bars with chord symbols and numerals | Progressions |
| F5 | Track Harmony | one track's own harmony: a sampled line's source, or a chord track's chords | the staff, the selected bars shaded | Progressions |

A section's name, its meter, adding and removing lines, and voice part
layers belong to F2 and are not built yet.

**Coupled and sampled are the composed line's form, not a mode.** A coupled
line has each note or chord with its own length; a sampled line has a sample
source and a rhythm line that takes from it (Song Notation, "Sample sources
and uncoupled lines"). The composed form is the musician's deliberate choice,
so the screen renders each line in its own form and labels it, leaving no
doubt what is composed:

- a coupled line is one staff, the notes with their values;
- a sampled line is three rows: its sample source as composed, labelled with
  where the source comes from and its play mode (*own, static* for every
  song today); its rhythm line; and a staff of the notes the rhythm takes
  from the source, in a quieter colour and labelled *derived*. Once a source
  marches, those notes are only where it starts (see Q9 and Q10).

An applied rhythm or progression goes where the line's form says: a rhythm
plays a coupled line's own notes or chords, or goes to a sampled line's
rhythm line; a progression goes onto a coupled line or into a sampled line's
source. Only a track with no line yet takes the lead's harmony. The form is
changed on purpose only, with C (F2) or a click on the track's form label,
asking first. Coupled to sampled keeps every note: each bar's source is shared into
as many even parts as it takes for every note to start on a part of its own.
Sampled to coupled composes the derived notes into the line; a source that
changes while a note sounds (track and hold) cannot be coupled, and the line
stays sampled.

**Player views (to come).** What a player sees is the player's choice, not
the composed form: a lead-sheet view (the play strip of EFS-1.5: the lead's
chord symbols, a rhythm line, the voicings at their changes) for playing an
instrument from the chords, and a voice view (the focused track in its
composed form, the lead's chords above) for playing or recording one part,
for example CV and MIDI to a modular synthesizer while the bass is focused.

The staff of F3 and F5 is the play strip of EFS-1.5: bars in play order,
repeats written out. The form strip across the top is always shown: every
entry as wide as it lasts, on a lane coloured by its key, with the playhead
(EFS-1.2).

**Browsing is free.** Moving through a library shows the row applied to the
selected bars at once, rendered as the song would be, marked PREVIEW. Nothing
is composed into the song until Enter applies it; Esc drops it. Space plays
the preview, so a progression or a groove can be heard in place before it is
applied.

**Intervals, not notes.** One set of keys moves whatever is selected, in every
mode: Shift+arrow a semitone, Ctrl+arrow a scale step, Alt+arrow an octave. A
scale step moves a note to the key's next note and a chord to the next degree's
own chord (`I` to `ii`, `IM7` to `ii7`), so the composer thinks in degrees and
the letters follow. Changing a section's key (F2, or K and F) moves its
composed notes with it; the chord numerals need no change, since they follow
the key.

**Apply** (Song Notation, section 7) is built as specified, rules 1 to 4: the
rhythm and the harmony can each come from the library or from what the line
already holds, so either can be swapped while the other stays. A bass line
takes the roots of the lead's chords only when asked (B), or when its track
has no line yet.

## Where the code is

- `crates/composer` (new): song authoring, with no window, file or MIDI. The
  document (text, song, undo), the outline of where entries, sections and
  lines are in the text, the edits, apply, interval moves, chord symbols, the
  libraries as browsed, the timeline the screen draws, and the session that
  takes commands. Apply's place was left open by Song Notation, section 8.
- `crates/render`: the editor's screen (`screen*.rs`), laid out from the
  session, with click regions; the spike's strip stays for `layout_preview`.
- `crates/app`: the window, the key map, the song file (saved, watched), and
  the sending thread (`playback.rs`, the event player's loop moved to its
  proper home, with Stop, a start place and looping).

## Left out of this draft

Beams (flags are drawn), slurs and proper ties, collision handling in chords,
cautionary key signatures, voice layers above 1, dynamics and cue lines, the
tempo slider, the cues panel, the warnings region, the text pane (the song
file can be edited in any editor and is reloaded), MusicXML, and the second
advance method (the strip uses the half shift). The window's hide/show staff
control of the spike is not carried over.

## Open questions

Every open question of this draft is here, labelled OPEN_QUESTION with its
number and the date and time it was raised. Each block gives the question,
its context, what was found, the options (a recommendation is not used until
Jim confirms it), and where and when it was raised.

**OPEN_QUESTION Q1 (2026-11-23 14:00):** Should an entry's own key also move
the notes its section composes?
- Context: the composition can play a section in another key
  (`"Chorus" key D major`). Chord numerals follow the key; composed notes do
  not (Song Notation, section 5: "The notes the section writes are not
  transposed").
- Found: in `songs_canonical/composer/composer_demo.song` the chorus,
  composed for C major, plays in D major with its bass still on F2 and C2
  against G and D chords; the staff shows the naturals. Reusing a section in
  another key works for the harmony and breaks the bass.
- Options: (a) an entry's key moves the section's composed notes by the
  interval between the keys; (b) a notation for notes as scale degrees (a
  bass of `1 5 1 3` that follows the key): composition by intervals in the
  file itself; (c) keep it and warn. The draft does (c) in F1 Arrange
  Sections, and moves notes with the key when the section's own key is
  changed (F2 Compose Sections).
- Raised: Claude, building the editor draft, 23 Nov 2026.

**OPEN_QUESTION Q2 (2026-11-23 14:00):** Should a track declare that its
pitch is fixed (a beat pulse), instead of the editor inferring it?
- Context: a beat pulse's pitch picks a device's gate, so moving a section's
  key must not move it.
- Found: the draft leaves alone any mono track that plays one and the same
  pitch throughout the section, the notation's own description of a beat
  pulse (Song Notation, section 5).
- Options: (a) keep the inference; (b) a track attribute such as
  `fixed_pitch`. Recommendation: (b), so nothing is guessed.
- Raised: Claude, building the editor draft, 23 Nov 2026.

**OPEN_QUESTION Q3 (2026-11-23 14:00):** May the render crate read the
composer's session for V1?
- Context: the Architecture says the render crate takes a surface and the
  published stream only.
- Found: the editor's screen is rendered from the composer's session (its
  timeline, selection and library rows), so `render` depends on `composer`,
  which has no window, file or thread.
- Options: (a) accept it for V1; (b) define the display stream as its own
  type first. Recommendation: (a), revisited before other clients.
- Raised: Claude, building the editor draft, 23 Nov 2026.

**OPEN_QUESTION Q4 (2026-11-23 14:00):** Are the starter progressions added
to `library/Progressions.library` correct?
- Context: the library held only the minor blues, so browsing had nothing to
  show.
- Found: the draft added the rest of the EFS-7.1 starter set: the 12-bar
  blues, quick change, jazz blues (with the ii–V into IV, the ♯iv°7 and the
  VI7 before the turnaround), 8-bar blues, ii–V–I major and minor, I–vi–ii–V
  and I–V–vi–IV.
- Options: Jim checks the forms, the jazz blues especially.
- Raised: Claude, building the editor draft, 23 Nov 2026.

**OPEN_QUESTION Q5 (2026-11-23 14:00):** Is it acceptable that an edit
rewrites the whole line in the song text?
- Context: every edit composes new song text, read back before it is taken.
- Found: the line an edit touches is composed again in a regular form (four
  bars to a text line, lengths only where they change, `/` for the same
  chord or note). Comments between that line's text lines are lost, and
  repeat signs (`|% %|`) and `euclid(...)` come back written out; a `fit`
  group that is not a triplet is refused with a message to edit the text.
  Lines the edit does not touch, and all comments around them, stay as
  composed.
- Options: (a) accept; (b) keep the line's own layout where its bars did not
  change, which is a larger piece of work.
- Raised: Claude, building the editor draft, 23 Nov 2026.

**OPEN_QUESTION Q6 (2026-11-23 14:00):** Should Esc no longer quit?
- Context: the spike quit on Esc.
- Found: in the editor Esc drops a preview or a bar range; Ctrl+Q quits, and
  asks again when there are unsaved edits.
- Options: (a) keep it so; (b) Esc quits when there is nothing to drop.
  Recommendation: (a), so a quick Esc never closes the editor.
- Raised: Claude, building the editor draft, 23 Nov 2026.

**OPEN_QUESTION Q7 (2026-11-23 14:19):** How should the staff move while
playing?
- Context: while editing, the selected section now starts at the left edge
  and a longer one turns whole pages from there; the strip moves only when
  the selection moves to another section. While playing, the strip still
  uses the half shift. Jim's goal for playing: fewer jumps, the eyes kept on
  one fixed place on the screen and the music scrolling into it.
- Found: the specification names two advance methods, both selectable while
  playing (EFS-1.8): *rewrite in place* and *half shift*. A steady scroll
  past a fixed reading place would be a third.
- Options: (a) a continuous scroll with the playhead fixed, for example a
  third of the way across, which keeps more than half the strip ahead
  (EFS-1.7); (b) rewrite in place, where nothing moves but the playhead;
  (c) keep the half shift. To be tried on the instrument.
- Raised: Jim, in the editor review, 23 Nov 2026.

**OPEN_QUESTION Q8 (2026-11-23 14:19):** Should the specification let the
musician choose how many bars fit across?
- Context: the specification sets bars across by screen size and
  resolution, down to a legible minimum (EFS-1.12).
- Found: the editor lets the musician make the staff's bars narrower or
  wider (Ctrl+- and Ctrl+=, about half to 2.4 times the usual width;
  narrower than half, eighth notes crowd).
- Options: (a) the specification allows it, within the legible minimum;
  (b) the editor only, not the player's strip.
- Raised: Jim's request for more bars in view, 23 Nov 2026.

**OPEN_QUESTION Q9 (2026-11-23 15:22):** How is a line composed that takes
its sample source from the lead?
- Context: a bass or other voice may take what it plays from the lead line
  instead of from a source of its own: the musician composes only its
  rhythm, and it follows the song's harmony. Every song marks one track as
  its lead, so such a line always has a source.
- Found: the notation has no way to compose this yet; a rhythm line with no
  `sample_source` line of its own is refused when the song is read. The
  editor's source row already names the origin (*own*); it would read
  *lead*.
- Options: the words in the song file (for example
  `line "Bass"  sample_source lead`), and what the line takes from a chord:
  its root, as B (bass from roots) does, or a chord tone chosen another way.
- Raised: Jim, in the coupled and sampled review, 23 Nov 2026.

**OPEN_QUESTION Q10 (2026-11-23 15:22):** What does each play mode of a
sample source do, and how is it composed?
- Context: today a source has one play head that moves bar for bar with the
  rhythm (Song Notation: "One play head at first"), so every source is
  *static*: its derived notes are known and shown. A source is meant to
  march: bar step, note step, pendulum, random, and others the musician
  defines.
- Found: the source row already names the mode (*static*). While a source
  marches, the derived notes shown are only its starting notes, labelled so.
- Options: to settle for each mode: what moves it along (each bar, each
  rhythm element, its own clock), where it starts, what it does at a
  section's repeat and at the section's end, and how it is composed in the
  song file.
- Raised: Jim, in the coupled and sampled review, 23 Nov 2026.
