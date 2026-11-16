# Song notation: what is decided and not yet in the code

15 Nov 2026, revised the same day with Jim's decisions. A working list, kept apart from `Song Notation.md` because that
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
| `dynamics` line | Loudness on a line of its own, `line "Keys" dynamics \| p:w \| < \| f \| mf \|`: `pp` `p` `mp` `mf` `f` `ff`, crescendo `<`, diminuendo `>`, `r` for no change. A level holds until the next mark. Straight-line levels, taken at the moment each element starts, rounded to the nearest whole velocity. A studio `accent` or `marcato` velocity is absolute (sections 5 and 8) | Not found. The loader reads only `voice_layer`, `sample_source` or the bars after a track's name, so a `dynamics` line is refused | **Ready for code.** Assumptions: the levels follow Song Notation's rule, a share of the room left from the track's normal velocity (`velocity normal`, 64 if the studio gives none): `f` 3/8 of the way to 127, `ff` 3/4, `p` 3/8 of the way to 1, `pp` 3/4, `mp` 3/16 (`mp` added on 15 Nov 2026, Jim: trivial, do it with the rest), rounded to the nearest whole velocity. At normal 64 the levels are 17, 40, 52, 64, 88 and 111 (`pp`, `p`, `mp`, `mf`, `f`, `ff`). One `dynamics` line per track per section, covering all its voice layers. A section with no `dynamics` line plays at `mf`; a section with one starts at `mf` and holds its last level to the section's end, so a level does not run on into the next section. A crescendo or diminuendo with no target level is refused, as the document says |
| `euclid(hits,steps)` | `euclid(3,8)` spreads hits over the bar's steps; a hit is a one-step sounding element, the others are rests; `C2:euclid(3,8)` and `:euclid(3,8)` for a source (section 3) | Not found. `trp` and `fit` (`fitQ`, `fitE`, `fitH`, `fitW`, `fitBar`) are built | **Ready for code.** Assumptions: the standard Euclidean (Bjorklund) pattern, with the first step a hit, so `euclid(3,8)` is hit, rest, rest, hit, rest, rest, hit, rest. The steps share the bar evenly, rounded as the source's elements are (`parse_source_bar`). Marks written after it apply to every hit. 0 hits, or more hits than steps, is refused with a message that says what to write |
| Repeat signs `\|%n … %\|` | A group of bars played again; `n` is the total number of plays; no number means twice (section 5, "Sections") | Not found. No `%` handling in the bar parser | **Ready for code.** Assumptions: the loader writes the bars out, as it does for `repeat`. Signs are written on a line and closed on the same line; no nesting (refused). Each line writes its own signs, and the line lengths are compared after the bars are written out. A length carried by a `/` or a left-off length, and a tie, continue across the sign as if the bars had been written twice. `n` below 1 is refused |
| Octave annotations `+8va`, `-8va`, `loco` | Move what follows one octave up or down from the clef's octave, at the start of any bar, on any track with notes or chords, until changed; `loco` returns. The `home_octave` setting is removed (section 4) | Not found. `home_octave` is still read by the loader and used by the planner, though the document says it was removed on 11 Nov 2026 | **Ready for code.** Assumptions: written as a word at the start of a bar's text, before its first element (`\| +8va C4:q D4 \|`); it sets the octave shift (absolute, not cumulative: `+8va` twice is still one octave up), applies to the elements after it on that line, and `loco` sets it to 0. Remove `home_octave` from the loader and the planner in the same step, and update every song and canonical file that uses it (a search finds them) |
| Chord inversions | `ii7[1stInversion]`, `[2ndInversion]`: the third (or fifth) is the lowest note; the notes that were below it move up one octave (section 4) | Not found | **Ready for code.** Assumptions: `[3rdInversion]` for a seventh chord, on the same rule; an inversion beyond the chord's notes is refused with the highest inversion that exists. Poly voices only, as other voicings are |
| `grouping` on a meter | `meter 7/8  grouping 2+2+3`: where the accents fall and how notes are beamed; the beat and tempo are unchanged (section 5) | Not found. A comment in `time_signature.rs` says it comes later | **Ready for code, in part.** Assumptions: parse and keep the grouping on the section, and refuse a grouping whose numbers do not add up to the pulses in the bar. It changes nothing in what is played (the accent is written by the author with `>`, as the beat pulse's is). The staff and LilyPond export use it later |
| One note for the same pitch in two voice layers | Note on at the first start, no second note on while it sounds, note off when the longest ends; the longest has priority (section 5, "Voice layers") | Not built. The planner puts every layer's notes into one track and does not merge equal pitches | **Ready for code.** Assumptions: only overlapping notes of the same pitch, on the same track, from different voice layers merge, from the earlier start to the later end. Notes that only touch are not merged: they follow the existing rules for touching notes |
| `meter_control` line: the meter changing bar by bar inside a section (decided 15 Nov 2026, Jim; syntax closed as Q-A) | Not in Song Notation. A section has one `meter`, so the common alternation of a 4/4 bar and a 5/4 bar cannot be written | **Decided (Jim, 15 Nov 2026; Q-A).** A `meter_control` line holds `mp` ("meter pattern") marks: `| mp [3_4 5_4 4_4] |` sets the meter of the next three bars, one per entry in the brackets. Underscore spelling keeps `/` free. `| mp section |` goes back to the section's own meter. A `/` cell plays the previous pattern once more; `/_end` plays it again and again to the end of the section. The line need not have as many bars as the other lines: bars it does not cover use the section's meter. The line is optional. Every other line of the section must have, in each bar, the length of that bar's meter. Consequence for the code: the planner and the staff can no longer assume one bar length for the whole section; the length of each bar comes from the section's list of bar meters. A carried tempo keeps the beats per minute (4/4 at 120 beats/min then 3/4 is still 120 beats/min); a meter change that changes the beat's length (4/4 to 6/8) needs a new anchor at that bar, one that keeps the same beat count per minute |
| Source line with its own play head and tempo | Question Q28; step 3 in the build order (section 8) | Not found. Every source moves with the rhythm | **Set aside (Jim, 15 Nov 2026):** part of the separate sample-source timing discussion, not this list |
| Controller for the sample source | The idea document `docs/Concept/CLAUDECODE_Decoupled_Sample_Controller_Idea.MD` | Idea only | **Set aside (Jim, 15 Nov 2026):** part of the separate sample-source timing discussion, not this list |

## Notation is accepted, but the player does not play it

| Item | What the code does today | Ready for code, or the decision needed |
|---|---|---|
| A different tempo in a later section | The loader accepts `tempo` in each section. The planner refuses a song whose sections differ in tempo ("Tempo changes between sections are not played yet") | **Ready for code** (a separate session is to take it up). Musical Time and Clock, Example 2, gives the method: a new anchor at the section's first position, the instant of that position computed from the previous anchor, and the pulses not yet queued recomputed from it. Assumption: an anchor is made at each section boundary where the tempo differs from the section before |
| A change of meter or key between sections | Read and kept per section; the planner uses each section's own bar length and key. Not tried by running a song. The `song_play_test` line that prints a place counts bars and beats in the first section's meter, so after a meter change its bar number is off | **Ready for code, and small.** Assumption: bars are numbered along the whole song, and each section counts its own bars at its own meter. Fix the place printed for each section. A meter change inside a section is a separate item, `meter_control`, in the first table |
| `settle_delay` in a studio file | Read; the player says "not applied yet" before it plays | **Dropped as a requirement (Jim, 15 Nov 2026).** The quantizer's own built-in delay function is used instead, so the program sends nothing late. Nothing to build. Still to clean up, one document at a time: the loader still reads `settle_delay` and the player prints "not applied yet"; Song Notation still lists it among the studio settings |

## Planned and refused by the loader

| Item | Today | Ready for code, or the decision needed |
|---|---|---|
| Track types `musician_voice` and `automation_control` | A song that names one is refused with "planned but not in V1" | **Deferred (Jim, 15 Nov 2026).** The loader keeps refusing them |
| Track type `tempo_control` | A song that names it is refused with "planned but not in V1" | **Needed (Jim, 15 Nov 2026).** Not deferred. Built together with the tempo marks below |
| Tempo marks inside a section | Not found. Song Notation (section 5, "Meter and tempo inside a section") has the decided notation | **Decided (Jim, 15 Nov 2026; Q-A). Ramps are required.** The line uses the same convention as `meter_control`: `| tp [130] |` sets the tempo to 130 beats/min at the start of that bar, a `/` cell holds it one more bar, and `/_end` holds it to the end of the section. `tp` takes a list, one tempo per bar, as `mp` does. Ramps (a tempo moving in a straight line over a number of bars) are required, so they are not deferred. The spelling is confirmed (`tp [110 ramp_to 130 2_bars]`, Jim, 15 Nov 2026), and the design is written in Musical Time and Clock, section 8: a ramp is a new anchor at every clock pulse, in a straight line over the score positions. Song Notation, Musical Time and Clock, the Experience Functional Specification and Architecture were brought up to date on 15 Nov 2026. A step needs the tempo anchors of the first table |

## Deferred, with no code

| Item | Today | Ready for code, or the decision needed |
|---|---|---|
| True polymeter (lines of different bar lengths playing against each other) | Every line of a section must have the same number of bars; a line of another length is refused | **Deferred (Jim, 15 Nov 2026).** The current rules stay: the rhythm line, the `sample_source` line and each voice layer of a section all have the same bar lengths |
| Navigation marks (D.C., segno, coda, first and second endings) | None | **Deferred (Jim, 15 Nov 2026).** The song's structure will be set in the user interface, and the sections that result encode it. No notation needed |
| Pressure and pitch bend | None | **Deferred (Jim, 15 Nov 2026).** They come in only if a MIDI record function is added (a real instrument is played and the result is kept as a line of the song); then pressure and pitch bend are kept with it. Otherwise they belong to `automation_control`, which is deferred too |
| More Strudel-style functions (repeat a step, play a group faster, weights) | None | **Deferred (Jim, 15 Nov 2026).** `/` and the `fit` functions cover the need for now |

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
8. `meter_control` (syntax decided, Q-A).
9. `tempo_control` with step marks, after item 7 gives the tempo anchors.
   Ramps follow, built as the clock design says: one new anchor per clock pulse.

## Open questions

**Q-A (CLOSED, Jim, 15 Nov 2026):** How are the `meter_control` and
`tempo_control` lines written? (Raised 15 Nov 2026 17:52, revised 18:20.)

- *Conclusion.* All of the proposal and the readings below are decided as
  written. The ramp spelling and the beats/s remark are decided too.

- *Context.* Jim's first example, `meter_control | 3/4,4/5-4 |`, could not
  be read with certainty. His second proposal (15 Nov 2026) is the one
  recorded here.
- *Proposal (Jim).* `| mp [3_4 5_4 4_4] |` sets the meter of the next three
  bars; `mp` means "meter pattern". `| mp section |` reverts to the
  section's meter. A `/` cell plays the previous `mp` pattern again;
  `/_end` plays it again and again to the end of the section. The number of
  bars on the line need not equal the number of bars of the other lines.
  `tempo_control` follows the same convention: `| tp [130] |`, then `/` to
  hold it, `/_end` to hold it to the end of the section. Ramps are required.
- *Readings (decided).* (1) A cell's width is its pattern's length, and a
  `/` plays the whole pattern once more, not one bar of it. (2) `mp section`
  is a pattern of one bar. (3) Bars not covered by the line use the
  section's meter, and a pattern cut by the end of the section is simply
  cut. (4) `tp` takes a list, one tempo per bar, like `mp`. (5) `meter_control` and `tempo_control` are lines of a
  section, not track types. (6) Ramps are required, not deferred, and
  need a spelling without `/`. Suggestion: the ramp is an entry in the
  `tp` list with its length in bars, `| tp [110 ramp_to 130 2_bars] |`
  (110 beats/min at the start, a straight line to 130 beats/min over two
  bars). *Spelling confirmed by Jim, 15 Nov 2026.* (7) Jim's remark "we can defer beats per second"
  is about the tempo unit only: `tempo 2 beats/s`, already written in
  Song Notation and in `Musical Time and Clock.md` section 13 for fast
  hardware sequencers, is not needed in the `tp` mark for now; `tp` takes
  beats/min. It has nothing to do with ramps.
- *Status.* Closed by Jim, 15 Nov 2026: "just convert the proposals to the decided directions." No open question remains in this document.
