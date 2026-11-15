# Song notation: what is decided and not yet in the code

15 Nov 2026. A working list, kept apart from `Song Notation.md` because that
document has grown long. It is not a specification. Each item says what
Song Notation says, what the code does today, and whether it can be coded
now. The code was checked on 15 Nov 2026 by searching the crates for the
feature and reading the loader, the planner and the player. "Not found"
means the feature's name or sign does not appear in the source. Nothing was
run to prove a negative.

**How to read the last column.** *Ready for code* means the specification
is detailed enough to build from, and any small gap is filled by the
assumption written there. The aim is good enough, not perfect: Claude builds
with the stated assumption, says so in the chat, and Jim corrects it if it
is wrong. *Decision needed* means something that changes what is built is
not settled, and the entry says what, with a recommendation.

## Decided, and not in the code

| Item | What Song Notation says | What the code does today | Ready for code, or the decision needed |
|---|---|---|---|
| `dynamics` line | Loudness on a line of its own, `line "Keys" dynamics \| p:w \| < \| f \| mf \|`: `pp` `p` `mf` `f` `ff`, crescendo `<`, diminuendo `>`, `r` for no change. A level holds until the next mark. Straight-line levels, taken at the moment each element starts, rounded to the nearest whole velocity. A studio `accent` or `marcato` velocity is absolute (sections 5 and 8) | Not found. The loader reads only `voice_layer`, `sample_source` or the bars after a track's name, so a `dynamics` line is refused | **Ready for code.** Assumptions: the levels are offsets from the track's normal velocity (`velocity normal`, 64 if the studio gives none): `pp` −48, `p` −24, `mf` 0, `f` +24, `ff` +48, clamped to 1 to 127 (the document's own example, `p` 40 and `f` 88 at normal 64, fits). One `dynamics` line per track per section, covering all its voice layers. A section with no `dynamics` line plays at `mf`; a section with one starts at `mf` and holds its last level to the section's end, so a level does not run on into the next section. A crescendo or diminuendo with no target level is refused, as the document says |
| `euclid(hits,steps)` | `euclid(3,8)` spreads hits over the bar's steps; a hit is a one-step sounding element, the others are rests; `C2:euclid(3,8)` and `:euclid(3,8)` for a source (section 3) | Not found. `trp` and `fit` (`fitQ`, `fitE`, `fitH`, `fitW`, `fitBar`) are built | **Ready for code.** Assumptions: the standard Euclidean (Bjorklund) pattern, with the first step a hit, so `euclid(3,8)` is hit, rest, rest, hit, rest, rest, hit, rest. The steps share the bar evenly, rounded as the source's elements are (`parse_source_bar`). Marks written after it apply to every hit. 0 hits, or more hits than steps, is refused with a message that says what to write |
| Repeat signs `\|%n … %\|` | A group of bars played again; `n` is the total number of plays; no number means twice (section 5, "Sections") | Not found. No `%` handling in the bar parser | **Ready for code.** Assumptions: the loader writes the bars out, as it does for `repeat`. Signs are written on a line and closed on the same line; no nesting (refused). Each line writes its own signs, and the line lengths are compared after the bars are written out. A length carried by a `/` or a left-off length, and a tie, continue across the sign as if the bars had been written twice. `n` below 1 is refused |
| Octave annotations `+8va`, `-8va`, `loco` | Move what follows one octave up or down from the clef's octave, at the start of any bar, on any track with notes or chords, until changed; `loco` returns. The `home_octave` setting is removed (section 4) | Not found. `home_octave` is still read by the loader and used by the planner, though the document says it was removed on 11 Nov 2026 | **Ready for code.** Assumptions: written as a word at the start of a bar's text, before its first element (`\| +8va C4:q D4 \|`); it sets the octave shift (absolute, not cumulative: `+8va` twice is still one octave up), applies to the elements after it on that line, and `loco` sets it to 0. Remove `home_octave` from the loader and the planner in the same step, and update every song and canonical file that uses it (a search finds them) |
| Chord inversions | `ii7[1stInversion]`, `[2ndInversion]`: the third (or fifth) is the lowest note; the notes that were below it move up one octave (section 4) | Not found | **Ready for code.** Assumptions: `[3rdInversion]` for a seventh chord, on the same rule; an inversion beyond the chord's notes is refused with the highest inversion that exists. Poly voices only, as other voicings are |
| `grouping` on a meter | `meter 7/8  grouping 2+2+3`: where the accents fall and how notes are beamed; the beat and tempo are unchanged (section 5) | Not found. A comment in `time_signature.rs` says it comes later | **Ready for code, in part.** Assumptions: parse and keep the grouping on the section, and refuse a grouping whose numbers do not add up to the pulses in the bar. It changes nothing in what is played (the accent is written by the author with `>`, as the beat pulse's is). The staff and LilyPond export use it later |
| One note for the same pitch in two voice layers | Note on at the first start, no second note on while it sounds, note off when the longest ends; the longest has priority (section 5, "Voice layers") | Not built. The planner puts every layer's notes into one track and does not merge equal pitches | **Ready for code.** Assumptions: only overlapping notes of the same pitch, on the same track, from different voice layers merge, from the earlier start to the later end. Notes that only touch are not merged: they follow the existing rules for touching notes |
| Source line with its own play head and tempo | Question Q28; step 3 in the build order (section 8) | Not found. Every source moves with the rhythm | **Decision needed.** The four open points of Q28 and the idea document's SC1 to SC12 are open. Recommendation: do not build Q28 alone. If the controller idea (below) goes ahead, Q28 is answered inside it |
| Controller for the sample source | The idea document `docs/Concept/CLAUDECODE_Decoupled_Sample_Controller_Idea.MD` | Idea only | **Decision needed:** Jim's go-ahead and SC11 (does the read position restart each pass?), SC12 (a controller line of a different length). The first piece that is clear without them is `source_shift` with diatonic, chromatic and octave shifts, on the assumption that a shift restarts at `base` each pass |

## Notation is accepted, but the player does not play it

| Item | What the code does today | Ready for code, or the decision needed |
|---|---|---|
| A different tempo in a later section | The loader accepts `tempo` in each section. The planner refuses a song whose sections differ in tempo ("Tempo changes between sections are not played yet") | **Ready for code** (a separate session is to take it up). Musical Time and Clock, Example 2, gives the method: a new anchor at the section's first position, the instant of that position computed from the previous anchor, and the pulses not yet queued recomputed from it. Assumption: an anchor is made at each section boundary where the tempo differs from the section before |
| A change of meter or key between sections | Read and kept per section; the planner uses each section's own bar length and key. Not tried by running a song. The `song_play_test` line that prints a place counts bars and beats in the first section's meter, so after a meter change its bar number is off | **Ready for code, and small.** Assumption: bars are numbered along the whole song, and each section counts its own bars at its own meter. Fix the place printed for each section |
| `settle_delay` in a studio file | Read; the player says "not applied yet" before it plays | **Decision needed, small.** The document says it is "time for a quantizer's voltage to settle after a note is sent" but not what moves. Recommendation: the Note On of each note on that track is sent that many milliseconds later than its place, and the Note Off with it, so the note keeps its length and only its start is late. Jim says if the device needs the opposite (the pitch sent first and the gate after) |

## Planned and refused by the loader

| Item | Today | Ready for code, or the decision needed |
|---|---|---|
| Track types `tempo_control`, `musician_voice`, `automation_control` | A song that names one is refused with "planned but not in V1" | **Not for now.** Not in V1; the notation says "Later" |
| Tempo marks `step` and `ramp` (section 5, "Tempo marks, in rhythm notation (later)") | Not found | **Ready for code once `tempo_control` and the tempo anchors above exist.** The notation is detailed enough; nothing else blocks it |

## Deferred, with no code

| Item | Today | Ready for code, or the decision needed |
|---|---|---|
| Polymeter | Every line of a section must have the same number of bars; a line of another length is refused | **Decision needed, only if wanted.** The idea document asks for one exception, controller lines, so it is part of that decision |
| Navigation marks (D.C., segno, coda, first and second endings) | None | **Decision needed.** Needs a design: where they are written and how the strip shows the order of play. Not blocking anything |
| Pressure and pitch bend | None | **Decision needed.** Needs a discussion of control tracks and continuous values first, as Song Notation says |
| More Strudel-style functions (repeat a step, play a group faster, weights) | None | **Not needed.** `/` and the `fit` functions cover them, as decided |
| `mp` on the dynamics line | None | **Ready, trivial,** when `dynamics` is built. Assumption: offset −12 from normal |

## Built on 15 Nov 2026, not yet in the "decided" list of section 8

Written into the documents on 15 Nov 2026 and built:

- `repeat 0`, which leaves a section out as if commented out.
- A section written again by name with `repeat n` and no lines.
- `/` on a rhythm line, which reads the sample source again at its own
  place.
- One line at the start of each play of a section in `song_play_test`.

## Checks still owed

- **To confirm before they are built** (Song Notation, section 8): the
  code names `Soft` and `Softer` for the accent levels of `<` and `<<`
  (they are built, so this is a name to confirm); whether `euclid(3,8)`
  may carry marks (assumed yes above).
- **To check against the Co-work decisions:** a `sample_source` line may
  not hold a rest, and a source pairs with the rhythm line of its own
  voice layer. Both are built as stated.
- **Not yet listened to:** the sections that play again by name, `repeat
  0` and the `/` change were checked only by tests and by the loader, not
  on the MIDI ports.

## A suggested order for the items that are ready

Smallest and most self-contained first; each is its own step, with its own
canonical files and tests, as the project does.

1. `grouping` (parse and keep), and the inversions.
2. Repeat signs `|%`.
3. `euclid`.
4. The merge of the same pitch across voice layers.
5. The `dynamics` line.
6. Octave annotations, together with the removal of `home_octave`.
7. Tempo changes between sections, and the meter-change place fix.
