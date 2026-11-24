# Live Lead Sheet — Editor Cheat Sheet

Revision 3 — 23 Nov 2026

The keys and buttons of the editor draft as built today
(`cargo run --release -p app`). Each one either **only changes what you
see**, or **changes the composed score**. Only the second kind changes the
song, and Ctrl+Z undoes it. The song file on disk changes only with Ctrl+S.

## The screen, top to bottom

| Part | What it is |
|---|---|
| Header | The song's name (`*` when there are unsaved edits); the five mode buttons F1 to F5; **Editing: …**, what an edit changes now; the play state and the MIDI ports. |
| Form strip | Every entry of the composition, as wide as it lasts, coloured by its key, with the playhead. A click selects the entry; on the staff it moves to the left edge. |
| Main view | The view of the mode. On the staff each track is labelled *coupled* or *sampled*; the label is also the control that changes it. |
| Library (right) | Progressions and Rhythms. Moving through it shows each row on the selected bars as a PREVIEW; nothing changes until you apply it. |
| Message line | What just happened, or what went wrong (red). Problems are also printed in the terminal, to copy whole. |
| Help lines | The keys of the current mode, and the keys that work in every mode. |

## The five modes

In the order of the song's hierarchy. Pressing a mode key or button never
changes the score.

| Key | Mode | Works on | View | Library tab |
|---|---|---|---|---|
| F1 | Arrange Sections | the composition: order, repeats, each entry's key and tempo, new sections | the composition as a list | Progressions (N: a new section) |
| F2 | Compose Sections | one section: its key and tempo, its lines' forms | the section's lines as lanes | Progressions (onto the selected line, as F5; to revisit) |
| F3 | Edit Lines | the music on a line, note by note | the staff | Rhythms |
| F4 | Song Harmony | the lead line's chords | every entry's bars with chord symbols and numerals | Progressions |
| F5 | Track Harmony | one track's own harmony: a sampled line's source, or a chord track's chords | the staff, the selected bars shaded | Progressions |

Not built yet in F2: a section's name, its meter, adding and removing lines,
voice part layers.

## When there is more than fits

There are no scroll bars. What does not fit is hidden, never squeezed, and
always named in red at the bottom right of the view: "Hidden for room: …".

| Where | What is kept in view | To see something hidden |
|---|---|---|
| F2 Compose Sections | The selected line and the lead, then the other tracks in the song's order; whole rows of bars, the selected bar's row kept in view | Up / Down to select a hidden track; Left / Right to a hidden bar |
| F3 Edit Lines, F5 Track Harmony | The selected track and the lead, then the others in order; the bars across the staff, the selected section at the left edge | Up / Down to select a hidden track; Ctrl+`-` for more bars across |
| F1 Arrange Sections, the library | The list follows the selection as it nears the bottom | Up / Down |

## Only changes what you see

| Key | Does |
|---|---|
| F1 to F5 | The mode |
| Left / Right | Next entry (F1), bar (F2, F4, F5), note (F3) |
| Up / Down | Next entry (F1, F4), line or track (F2, F3, F5) |
| Shift+Left / Right | Widen / narrow the selected bars |
| Ctrl+`-` / Ctrl+`=`, or Ctrl+wheel | Staff: more / fewer bars across |
| Click | Select an entry, bar, track, note, mode, tab or library row |
| Tab | Between the main view and the library |
| Up / Down in the library, or the wheel over it | Browse; each row shows as a PREVIEW |
| Esc | Drop the preview, or the bar range |
| Space | Play from the selected entry, or stop; plays the preview if one is shown |
| Shift+Space, L | Play from the start; loop the selected entry |
| Ctrl+Q | Quit; asks again if there are unsaved edits |

## Changes the composed score

Apply from the library:

| Key | Does |
|---|---|
| Enter, or double-click a library row | Apply the preview to the selected bars |
| N | A new section after the entry, from the progression under the library cursor |

Move by intervals; what moves depends on the mode:

| Key | F1 | F2 | F3 | F4 | F5 |
|---|---|---|---|---|---|
| Shift+Up / Down | entry key, a semitone | section key, a semitone, notes with it | the note, a semitone | the lead's chords, a semitone | the track's harmony, a semitone |
| Ctrl+Up / Down | entry key, a fifth | section key, a fifth, notes with it | the note, a scale step | the lead's chords, a degree | the track's harmony, a scale step or degree |
| Alt+Up / Down | — | — | the note, an octave | the lead's chords, an octave | the track's harmony, an octave |
| Ctrl+Shift+Up / Down | — | — | the note, to the next tone of the lead's chord | — | — |

One note at a time (F3):

| Key | Does |
|---|---|
| R | Rest to note (the lead's chord root there; the chord on a chord track), or note to rest; same length |
| A | Accent, marcato, none |
| B | The selected bars of a mono track play the roots of the lead's chords |

Sections and lines:

| Key | Does |
|---|---|
| C, then C again | Change the track's line in this section between coupled and sampled; the first press only asks. Every note sounds as before. |
| K / Shift+K | Section key up / down a semitone, its notes moving with it (F1: the entry's own key; notes do not move) |
| F / Shift+F | The same, round the circle of fifths |
| T / Shift+T | Tempo up / down 5 beats/min (F1: the entry's own tempo) |

The composition (F1):

| Key | Does |
|---|---|
| `[` / `]` | One repeat fewer / more |
| M | Mute the entry (repeat 0), or restore it |
| D | Duplicate the entry after itself |
| Delete | Remove the entry |
| Alt+Left / Right | Move the entry earlier / later |

Undo and save:

| Key | Does |
|---|---|
| Ctrl+Z, Ctrl+Y (or Ctrl+Shift+Z) | Undo, redo any change to the score |
| Ctrl+S | Write the song file to disk |

## Watch out: where composed notes are replaced

- **B** replaces the notes of the selected bars with the lead's roots; that
  is its purpose.
- **Applying a progression** replaces the chords of the selected bars: the
  lead's in F4, the selected track's in F2 and F5.
- **K and F** (and the interval keys in F2) move every composed note of the
  section. In F1 they change the entry's own key and do not move notes, so a
  section played in another key keeps its bass notes.
- **Any edit rewrites the whole line in the song text.** Comments between
  that line's text lines are lost, and repeat signs and `euclid` come back
  written out. Other lines are left as composed.

Applying a rhythm no longer replaces composed notes: it plays the line's own
notes or chords. Only a track with no line yet takes the lead's harmony.

## How a track is rendered on the staff

| Form | Rendered as |
|---|---|
| Coupled (each note with its own length, `C2:h G2`) | One staff: the notes and their values |
| Sampled (a sample source and a rhythm line) | Three rows: the source as composed, labelled "source: own, static"; the rhythm line; the staff of derived notes, in green, labelled "derived" |

| Colour | Means |
|---|---|
| Blue box | The selected note (F3) |
| Lighter blue | The selected bars |
| Faint yellow | The bars of the selected section |
