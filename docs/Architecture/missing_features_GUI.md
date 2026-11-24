# Live Lead Sheet — Missing features of the editor (GUI)

Revision 1 — 23 Nov 2026

What Song Notation (Revision 27) lets a song file say, against what the
editor draft can render and compose today. A song that uses any of these
still loads, plays and can be edited by hand in any text editor; the gap is
only in the program's screen and keys.

Each row says whether the editor **renders** the feature (shows it as
composed) and whether it can **compose** it (create or change it with its
own keys and library). "Partly" is explained in the note. The last column
names the mode where it would belong (F1 Arrange Sections, F2 Compose
Sections, F3 Edit Lines, F4 Song Harmony, F5 Track Harmony), as a first
guess for the order of work, not a decision.

Related open questions are in `Editor Draft.md`, section "Open questions".

## Rules shared by all files (Song Notation, section 2)

| Feature | Renders | Composes | Note | Mode |
|---|---|---|---|---|
| Comments (`#`, `---`) | No | No | Comments are kept in lines an edit does not touch; the line an edit rewrites loses the comments between its text lines (Q5). | — |
| Errors shown while the program runs | Partly | — | An edit that would not load is refused with the reader's message. But a song file with mistakes stops the program at start-up, where Song Notation says the program still starts, shows the errors and checks the file again on each save; a song changed outside into error is shown as a problem and not loaded. | — |
| A plain text pane in the editor | No | No | Song Notation says the editor includes one. The file can be edited in any editor today and is loaded again within a second. | — |
| Every chord type of the table (power chord to thirteenth) | Yes | Partly | Applied from the progression library, moved by degree or semitone. A chord type cannot be chosen directly (for example turning `V` into `V7`). | F4, F5 |

## Rhythm notation (section 3)

| Feature | Renders | Composes | Note | Mode |
|---|---|---|---|---|
| Note values `w h q e sx tx`, up to three dots | Partly | Partly | Rendered with flags, not beams (EFS-1.5 asks for beams grouped by the meter). Double and triple dots are not checked on screen. Lengths come only from an applied rhythm; a single note cannot be made longer or shorter, split or merged. | F3 |
| Rests | Yes | Partly | R turns a note into a rest of the same length and back. A rest cannot be lengthened or split. | F3 |
| `trp` (triplets) | Partly | Partly | The triplet's 3 is shown as a small mark, with no bracket. Triplets come only from an applied rhythm. | F3 |
| `fitQ` `fitE` `fitH` `fitW` `fitBar` | Partly | No | Rendered as the notes they give, without a bracket or number; an edit of a line holding one that is not a triplet is refused, with a message to edit the text. | F3 |
| `euclid` | Partly | No | Rendered as the notes it gives; an edit writes it out as notes (Q5). | F3 |
| Notes inside a function (`trp(F2:q A2:q C2:q)`) | Partly | No | As the functions. | F3 |
| `/` the same again | Yes | Partly | Rendered as the notes it plays; an edit writes `/` where the same note or chord follows. | F3 |
| Ties `~`, across barlines | Partly | No | Drawn as a short line, not a tie arc; kept by edits; R removes a tie into or out of a rest. A tie cannot be added or removed. | F3 |
| Accent `>`, marcato `>>` | Yes | Yes | A cycles none, accent, marcato. | F3 |
| Softer `<`, `<<` | — | No | No sign on the staff, as specified. No key sets them. | F3 |
| Staccato `'` | Yes | No | Shown as a dot mark. No key sets it. | F3 |
| Legato `_` | No | No | No slur is drawn. No key sets it. | F3 |
| Fermata `^` | No | No | No fermata sign. No key sets it. | F3 |
| A rhythm assigned to a bar range | Yes | Yes | Shift+Left/Right, then a rhythm from the library. | F3 |

## Elements, contents and marks in a line (section 4)

| Feature | Renders | Composes | Note | Mode |
|---|---|---|---|---|
| Chord names in the key | Yes | Partly | From the progression library; moved by degree (Ctrl) or semitone (Shift); no direct choice of chord. | F4, F5 |
| Notes in brackets (`[D3 F3 A3 C4]`) | Yes | No | Rendered; no way to compose a voicing note by note. | F3, F5 |
| A chord with its own voicing (`ii7[F3 A3 C4]`) | Yes | Partly | Rendered with its notes; moving it by degree moves the chord and the notes. | F5 |
| Inversions (`ii7[1stInversion]`) | Yes | Partly | Rendered; kept when the chord moves. No key chooses an inversion. | F5 |
| Mono notes (`C2`, `F#2`) | Yes | Partly | Moved by semitone, scale step, chord tone or octave; a rest becomes the lead's root (R). A note cannot be typed or picked by its letter. | F3 |
| Cue names on a cue line | No | No | Cue lines are not rendered. | F3 |
| `:q` elements of a rhythm line | Yes | Yes | Shown on the rhythm line of a sampled track; applied rhythms go there. | F3 |
| A beat pulse (one fixed pitch) | Yes | Partly | Left alone when a section's key moves (inferred, Q2). | F3 |

## The song (section 5)

### Song, tracks and voice layers

| Feature | Renders | Composes | Note | Mode |
|---|---|---|---|---|
| Tracks: add, remove, rename, order | Partly | No | Rendered in the song's order. No editing. | — (a song mode, not yet named) |
| Track types `poly_voice`, `mono_voice` | Yes | No | Shown in F2's lane labels. | — |
| `clock_and_transport` | — | No | Not shown; it has nothing to render. | — |
| `cue_control` track and its cues | No | No | Not rendered; cues cannot be declared or placed. | F2, F3 |
| `meter_control` track type | No | No | Not rendered as a track; see below. (`tempo_control` was removed from the notation in Revision 27: tempo marks go on the lead line.) | F2 |
| `musician_voice`, `automation_control` | No | No | Deferred in Song Notation. | — |
| Clefs treble, bass, grand, percussion | Yes | No | The clef cannot be changed. | — |
| Clefs alto and tenor | Partly | No | Notes are placed for the clef, but the G clef glyph is drawn instead of the C clef. | — |
| `lead` (exactly one track) | Yes | No | The lead's chords give the chord symbols; it cannot be moved to another track. | — |
| `chord_numeral_standard` | — | Partly | Written by the editor when an applied progression needs it; not shown or chosen. | F4 |
| Octave annotations `+8va` `-8va` `loco` | Partly | No | Notes sound and are rendered at the shifted octave, but no 8va line is drawn and none can be added. | F3 |
| Voice layers above 1 (`voice_layer 2`) | No | No | Only layer 1 is rendered and composed; BWV 610 shows one layer per track. | F2, F3 |
| `voice_layer n "Name"` | No | No | | F2 |

### Sections and the composition

| Feature | Renders | Composes | Note | Mode |
|---|---|---|---|---|
| Section name | Yes | No | Cannot be renamed. | F2 |
| Section `key` | Yes | Yes | K, F, and the interval keys in F2; notes move with it. | F2 |
| Section `meter` | Partly | No | The time signature is drawn at the start of the strip only, not again where the meter changes. Cannot be changed. | F2 |
| Beat grouping (`grouping 2+2+3`) | No | No | Not used for beaming or accents on screen. | F2 |
| Section `tempo` | Yes | Yes | T / Shift+T. Fractional tempos and beats/s are not composed. | F2 |
| `count-in` | Yes | No | Shown in the form list; cannot be set or removed. | F1 |
| Sections no entry names | No | No | A section the composition does not play is not shown anywhere. | F1 |
| A new section | — | Partly | N makes one from a progression; there is no empty new section and no copy of a section under a new name. | F1 |
| Entries: order, repeat, mute, duplicate, remove | Yes | Yes | Alt+Left/Right, `[` `]`, M, D, Delete. | F1 |
| An entry naming an existing section | — | Partly | Only by duplicating an entry; no choice of section for a new entry. | F1 |
| Entry `key` and `tempo` | Yes | Yes | K, F, T and the interval keys in F1. The entry's key does not move composed notes (Q1). | F1 |
| `key` / `tempo` lines between entries (a new base) | Partly | No | Their effect is shown on the entries after them; they cannot be added, changed or removed. | F1 |
| The composition's starting key, meter, tempo | Partly | No | Shown through the entries; not editable. | F1 |
| Repeat signs `|%` `%|`, `|%3` | Partly | No | The bars are rendered written out; an edit of the line writes them out in the file (Q5). | F3 |
| Polymeter | — | — | Deferred in Song Notation. | — |
| A long line over several text lines | — | Partly | Kept where an edit does not touch; an edited line is written four bars to a text line (Q5). | — |

### Sample sources and uncoupled lines

| Feature | Renders | Composes | Note | Mode |
|---|---|---|---|---|
| `sample_source` line | Yes | Partly | Rendered as the source row; a progression applied in F5 or F2 goes into it; C converts a line between coupled and sampled. Its chords or notes cannot be composed one by one. | F5 |
| The source's even shares of the bar | Yes | Partly | Set by the number of chords applied or by C's conversion; no key adds or removes a share. | F5 |
| Track and hold (the source changing while a note sounds) | Partly | — | The derived staff shows the note at its start only; C refuses to convert such a line to coupled. | F5 |
| A line taking the lead as its source | — | — | Not in the notation yet (Q9). | F5 |
| Source play modes (marching) | — | — | Not in the notation yet (Q10); every source is labelled *static*. | F5 |

### Dynamics, cues, meter and tempo inside a section

| Feature | Renders | Composes | Note | Mode |
|---|---|---|---|---|
| `dynamics` lines (`pp` to `ff`, `<` `>` ramps) | No | No | Not rendered; Song Notation places them above the staff, below the chords. | F3 |
| Cue lines (`"Start SQ-1":w`) | No | No | See cue_control above. | F3 |
| `meter_control` pattern (`mp [3_4 5_4]`) | Partly | No | Bars take their meters, but no time signature is drawn for the changing bars. | F2 |
| Tempo marks on the lead line (`tempo 130 beats/min`) | No | No | Not shown on the staff or in the form; played by the player. | F2, F4 |
| Tempo ramps (`ramp_to … 2_bars`) | No | No | As tempo marks. | F2, F4 |

## The studio configuration (section 6)

The editor reads the studio file only to open its MIDI ports and play. None
of it is shown or edited in the program: ports, each track's port and
channel, `octave_offset`, `velocity` (`mezzo_forte`, `accent`, `marcato`),
`retrigger_gap`, `delay_trigger`, and destinations. The studio file is edited
by hand, and `tools/tune_delay.py` measures and writes the delays.

## The libraries and apply (section 7)

| Feature | Renders | Composes | Note | Mode |
|---|---|---|---|---|
| Browsing both libraries, grouped | Yes | — | Rhythms grouped by meter then group, those of another meter greyed; progressions by idiom and kind. | all |
| Apply rules 1 to 4 | Yes | Yes | A rhythm plays a coupled line's own notes or chords; a track with no line takes the lead's harmony (rule 3, roots on a mono voice). | F3, F4, F5 |
| Rule 4: more chords than sounding elements | Partly | — | The extra chords are left off; the message names the bar, but nothing asks the musician where to place them. | F4 |
| Rule 5: a harmony line without the root | No | No | Applying a progression always writes the full chord. | F5 |
| Adding or editing library entries | No | No | The library files are edited by hand. | — |
| Apply as a drag | No | No | Song Notation says "in the editor it is a drag"; the draft applies with Enter or a double-click. | — |

## Rendering asked for by EFS-1.5 and not drawn yet

These are not notation features but are what the staff should show:
beams grouped by the meter, tie arcs, slurs for legato, triplet brackets,
cautionary key signatures, collision handling for seconds in a chord, the
chord symbols alternating rows where they would touch, and voicings shown
only at change points in the lead-sheet layout (EFS-1.5's play strip).
