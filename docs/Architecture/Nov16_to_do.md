# Song notation: what is decided and not yet in the code

16 Nov 2026, revised from the list of 15 Nov 2026 with Jim's decisions. A working list, kept apart from `Song Notation.md` because that
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
| --- | --- | --- | --- |
| **meter_control** line: a repeating meter pattern for a section | "Meter Control" (section 5). A section has either a `meter` or a `meter_control` line. The line holds one `mp` mark, `\| mp [3_4 5_4 4_4] \|`, a pattern of three bars that repeats to the end of the section. The section's length is a multiple of the pattern's. Every meter of the pattern has the same beat, so the tempo in beats per minute is the same in every bar | Not found. A section has one `meter`, so the alternation of a 3/4 bar and a 5/4 bar cannot be written | **Ready for code (Jim, 16 Nov 2026).** Consequence for the code: the planner, the counter, the staff and the line checks can no longer assume one bar length for a section; the length of each bar comes from the section's list of bar meters. Inside a section the beat never changes, so the pattern makes no tempo anchor. A section that writes neither a `meter` nor a `meter_control` line carries the one before. The first bar of a section with a pattern uses the pattern's first meter for the beat of `tempo`. The loader refuses: a pattern whose meters differ in beat, a section whose length is not a multiple of the pattern's, a second `meter_control` line, and a section with both a `meter` and a `meter_control` line |
| **Tempo marks on the lead line**, with ramps | "Tempo marks on the lead line" and "Tempo ramp" (section 5). A bar of the lead track's first voice layer may start with `tempo 130 beats/min`; the tempo holds until the next mark. A ramp, `tempo 110 ramp_to 130 beats/min 2_bars`, moves in a straight line and then holds. Each play of a repeated section starts at the section's tempo | Not found. The planner changes the tempo only between sections | **Ready for code (Jim, 16 Nov 2026).** The planner makes one anchor per mark, at its bar. A ramp makes one anchor at every clock pulse, in a straight line over the score positions (Musical Time and Clock, section 8). The loader refuses: a mark on any other line or layer, a mark in a section with no lead line, two marks in a bar, a mark inside a ramp, and a ramp that does not finish inside its section |
| **Minor keys: `key C minor`** | `minor_key_treatment_decision.md`, Conclusion. A key is a tonic and major or minor; a minor key takes its relative major's signature | `Mode` has only `Major`; the parser refuses `key C minor` | **Done (16 Nov 2026).** Add `Mode::Minor`, accept it in the parser, give it the relative major's signature. A song in any key must load |
| **Chord numeral practice in minor** | Same document. A song that uses numerals declares `chord_numeral_standard key_dependent` or `major_referential`; a minor song with numerals and no line is refused | Not found. The voicing counts steps from the major scale only | **Done (16 Nov 2026).** Read the line; find each numeral's root by the declared practice; refuse the omission with a message that says what to add |
| Note spelling in minor keys | Same document, item 2: flats or sharps from the key | Spelling takes the major key only | **Done (16 Nov 2026).** A minor key spells like its relative major |
| Chord libraries: numeral standard line and style rule | Same document, Conclusion: one `chord_numeral_standard` line per numeral library; one style per file | Not found | **Done (16 Nov 2026):** a progression library loader for Roman numerals, with `mode major` or `mode minor` on each entry and the `chord_numeral_standard` line required when any entry is minor. Apply, the starter library file and the warning on applying an entry to a song of the other kind (for the text player only) are not built |
| Letter chords in song lines and in libraries | Minor key document, Conclusion: libraries may name chords by letter (`Am`, `E♭maj7`) and give a bass note (`Am/G`). A letter chord names its root outright, so it needs no key, `mode` or `chord_numeral_standard` | Not found. Song lines take Roman numerals and notes only; no progression library loader exists | **Not scheduled.** Song Notation does not yet say where a letter chord is written in a song line. The progression library loader is built first for Roman numerals; a letter chord in a library entry is refused with a message until this item is done |
| LilyPond export of a minor key (`\key … \minor`) | Same document, item 2 | The export writes major keys only | **Deferred (Jim, 16 Nov 2026)** to the end of the project |
| Source line with its own play head and tempo | Section 8, build order step 3 | Not found. Every source moves with the rhythm | **Set aside (Jim, 15 Nov 2026):** part of the separate sample-source timing discussion, not this list |
| Controller for the sample source | The idea document `docs/Concept/CLAUDECODE_Decoupled_Sample_Controller_Idea.MD` | Idea only | **Set aside (Jim, 15 Nov 2026):** part of the separate sample-source timing discussion, not this list |

## Planned and refused by the loader

| Item | Today | Ready for code, or the decision needed |
| --- | --- | --- |
| Track types `musician_voice` and `automation_control` | A song that names one is refused with "planned but not in V1" | **Deferred (Jim, 15 Nov 2026).** The loader keeps refusing them |
| Track type `tempo_control` | A song that names it is refused with "planned but not in V1" | **Removed (Jim, 16 Nov 2026).** Tempo changes are marks on the lead line. The loader refuses it as it refuses any name that is not a track type |

## Deferred, with no code

| Item | Today | Ready for code, or the decision needed |
| --- | --- | --- |
| True polymeter (lines of different bar lengths playing against each other) | Every line of a section must have the same number of bars; a line of another length is refused | **Deferred (Jim, 15 Nov 2026).** The current rules stay: the rhythm line, the `sample_source` line and each voice layer of a section all have the same bar lengths |
| Navigation marks (D.C., segno, coda, first and second endings) | None | **Deferred (Jim, 15 Nov 2026).** The song's structure will be set in the user interface, and the sections that result encode it. No notation needed |
| Pressure and pitch bend | None | **Deferred (Jim, 15 Nov 2026).** They come in only if a MIDI record function is added (a real instrument is played and the result is kept as a line of the song); then pressure and pitch bend are kept with it. Otherwise they belong to `automation_control`, which is deferred too |
| More Strudel-style functions (repeat a step, play a group faster, weights) | None | **Deferred (Jim, 15 Nov 2026).** `/` and the `fit` functions cover the need for now |

## A suggested order for the items that are ready

1. `meter_control`.
2. Tempo marks on the lead line, as steps and ramps. Done 16 Nov 2026.
3. Minor keys: `key C minor`. Done 16 Nov 2026.
4. Chord numeral practice in minor. Done 16 Nov 2026.
5. Note spelling in minor keys, then the progression library loader (both done 16 Nov 2026).
6. The GUI. LilyPond export of minor keys waits until the end of the project.


**JIm to update:** 
For you to edit, in Song Notation. Two examples still show the old wording:

Line 283: charleston.rhythm line 3: bar 1 is short by an eighth note. Add e or er. would become charleston.rhythm line 3: bar 1 is 4/4 and short by an eighth note. Add e or er.
Line 662: line 34: line "Keys": bar 7 is short by a quarter note would become line 34: line "Keys": bar 7 is 4/4 and short by a quarter note.
