# Live Lead Sheet — Song Notation Clarifications

11 Nov 2026. Clarifications Claude needs on Jim's responses in section 9,
"Open questions", of `docs/Architecture/Song Notation.md` (Revision 18).
Line numbers are those of that file as it stood on 11 Nov 2026.

Each block is labelled OPEN_QUESTION and has a **Jim's answer** line to
fill in. "Used unless you say otherwise" is what Claude will write into
the document if the answer line is left empty.

---

**OPEN_QUESTION C-Q14:** With no timing in a progression, where does
apply put the second chord of a two-chord bar?

- Refers to: **Jim's response Q14**, line 814.
- Context: you decided a chord library carries no rhythm, only sequences
  (chords in a bar, over several bars). A library bar such as
  `| ii7 V7 |` still has two chords, and apply must put the second one
  on some element of the rhythm.
- Why it matters: it fixes what apply writes for every bar with more
  than one chord.
- Options:
  - (a) Apply spreads a bar's chords evenly across the bar, as its own
    rule; you move them by hand afterwards.
  - (b) Each chord goes on the next sounding element in order.
- Used unless you say otherwise: (a). The `@` spelling and Appendix A,
  "A chord's intended place inside a library bar", are dropped.
- Raised: Claude, 11 Nov 2026.

  **Jim's answer C-Q14**
  - The note will sample the chord based on its distribution in the bar
  - Example 3 chords in a bar with 4/4; given a sounding element in the timing of the second dotted   time element; the note is sampled and used; I.e. let the rhythm sample the chords or notes based on the distribution of the chords or notes.
  - critical conceptual clarification: Per our discussion of modular decoupling of notes and rhythm: the solution will  evolve such that the chord or notes will have their own play heads independently of a rhythm track; VERSION ONE OF THIS PROGRAM WILL NOT SUPPORT THAT CAPABILITY.  The important thing is to realize once the bards in a chord track have their own tempo, the rhythms can SAMPLE the note at that moment from its assigned note or chords; this decoupling will be a key feature. First though we need standard music to work; so the chords are split evenly (4/4 time with 4 chords each occupies a q, 3/4 time with 2 chords each gets 2/3. The important thing to understand is that Rhythm will sample notes or chords.  Version one will have one common play head at the tempo of the song. Version 2 will allow for the free decoupling so that this product becomes a true modular sequencer vs just another guitar-pro or similar app.  
  Let's answer the remaining questions; this topic alone could consume an entire session; I am trying
  to convey a simplifying concept : that the rhythm samples from an evenly distributed chord source.

  - to make this work we simply introduce the idea that the first part may not carry a timing element
   ```
  line "Harmony"  | I               | IV      IV    V    |   --- pure harmony, evenly distributed
                  | :e.             | :e      :q         |   --- pure rhythm, sampled from above line
                                                             --- aimed at modular ideas about music
                                                             --- tempo for the harmony line may not equal that of 
                                                                 rhythm line; creates variation in music

  or our existing harmony line
  
                  | I:e. V          | IV7     IV         |   --- coupled composed; no sampling, still valid!

  SO each part can be coupled or uncoupled
        
  line "Bass"     | C2 E2 G2        | F2   A2  C3        |   --- uncoupled, notes only
                  | :q :e           | F1   A2  C3        |   --- uncoupled sample from 

  line "Horn"
                  | C2> E2 G2       | F1>  A2  C4        |   --- coupled

   ```

**OPEN_QUESTION C-Q15:** Do `+8va` and `-8va` replace the track
attribute `home_octave`, and how does an octave annotation end?

- Refers to: **Jim's response Q15**, line 832.
- Context: you decided the root of a chord is placed by the clef, and
  that `+8va` / `-8va` move the octave relative to the clef, from any
  bar start, on any track with notes or chords. Section 5, "The song
  (`.song`)", also has an optional track attribute `home_octave` that
  does the same job for a whole track.
- Why it matters: two ways to set the octave would have to be kept in
  step, and an annotation with no end rule plays unpredictably.
- Options:
  - (a) `+8va` / `-8va` replace `home_octave`; the clef alone sets the
    octave (treble 4, alto 3, tenor 3, bass 2, grand 4).
  - (b) Both are kept.
  - End rule: an annotation lasts until the next one or the end of the
    section; or it needs an explicit end sign.
- Used unless you say otherwise: (a), with the annotation lasting until
  the next one or the end of the section.
- Raised: Claude, 11 Nov 2026.

  **Jim's answer C-Q15**
  Yes - marks are in effect until otherwise stated 
      - home octave is derived from the cleff
      - the studio file offers an offset (already supported, I believe
---

**OPEN_QUESTION C-Q17a:** Is the base velocity of 64 fixed, or a default
that a `.studio` file can change?

- Refers to: **Jim's Response Q17**, line 862.
- Context: you decided `>`, `>>`, `<` and `<<` are relative to a base
  velocity (you think 64). Your decision of 5 Nov 2026 (section 1, "The
  model in brief") puts how hard an accent plays per device in the
  studio configuration, not in the song.
- Why it matters: devices respond differently to velocity, most of all
  a velocity gate on the FH-2.
- Options:
  - (a) 64 is the default base, and the base and the four steps (`<<`,
    `<`, `>`, `>>`) are adjustable per track in the `.studio` file.
  - (b) The base and the steps are fixed in the player.
- Used unless you say otherwise: (a).
- Raised: Claude, 11 Nov 2026.

  **Jim's answer C-Q17a**
  - Yes, < and > and << and >> are relative to 64 unless volume_normal is defined in the .studio
  - each .studio entry may indicate the absolute values for < and > and << and >> 
---

**OPEN_QUESTION C-Q17b:** What do `<` and `<<` draw on the staff, and
what are they called?

- Refers to: **Jim's Response Q17**, line 862.
- Context: `>` draws the accent sign and `>>` the marcato sign. Standard
  rudiments has no articulation sign for a single soft note, and in
  print `<` is the crescendo hairpin.
- Why it matters: the mark must read correctly on the staff and on an
  exported score.
- Options:
  - (a) A parenthesised note head, the ghost-note convention of drum
    notation.
  - (b) A dynamic letter under the note (`p`, `pp`).
  - (c) Nothing on the staff; the mark only changes the velocity.
- Used unless you say otherwise: nothing; this one needs your choice.
- Raised: Claude, 11 Nov 2026.

  **Jim's answer C-Q17b**
  I indicatated standard notation; there are many markings let's go with
  the most common. 
  << means pp pianissimo(in the player we will display italic pp)
  < means p piano
  > forte
  >> fortissimo
  these normally are positioned around bars and phrases

  The issue I see is that the gradual crescendo marks are written with an elongated > or <
  We should write >... and ...< in the text file to note this, on a line between the upper staff line or ledger line,
  and below the chords.  In the live player UI we can render this as elongated symbols.

---

**OPEN_QUESTION C-Q20:** When one voice layer holds a pitch and another
voice layer of the same track strikes that pitch part-way through, is
the note struck again?

- Refers to: **Jim's response Q20**, line 913.
- Context: you decided the note off is sent only when the longest of the
  notes has finished. That settles the end. It does not settle the
  second start: a note on for a pitch that is already sounding.
- Why it matters: devices differ in what they do with a second note on
  for a held pitch, so the result would not be predictable.
- Options:
  - (a) No second note on. One note sounds from the first start to the
    last end.
  - (b) The note is struck again: note off, then note on, at the second
    start.
- Used unless you say otherwise: (a).
- Raised: Claude, 11 Nov 2026.

  **Jim's answer C-Q20**
  -The intended effect would be legato (so no new gate)
  -Can midi decouple this?

---

**OPEN_QUESTION C-Q21:** Do you want `%`, repeat signs inside a section,
or both?

- Refers to: **Jim's response Q21**, line 930.
- Context: Q21 asked about `%`, the sign for "play the previous bar
  again". Your response covers repeat signs: a start and an end sign
  around a group of bars. Today a repeat is written only for a whole
  section, with `repeat n` (section 5, "The song (`.song`)",
  subsection "Sections").
- Why it matters: they are two different pieces of notation and loader
  code.
- Options:
  - (a) `%` only.
  - (b) Repeat signs around a group of bars inside a section only.
  - (c) Both.
  - (d) Neither for now; `repeat n` on a section is enough.
- Used unless you say otherwise: nothing; Claude cannot tell which you
  meant.
- Raised: Claude, 11 Nov 2026.

  **Jim's answer C-Q21**
  - 

---

**OPEN_QUESTION C-Q23:** Which functions from Appendix B are to be
built?

- Refers to: **Jim's response Q23**, line 950.
- Context: you decided functions are allowed in `.song` files and that
  Appendix B, "What Strudel's mini-notation offers", is to be
  implemented as discussed. Appendix B is a comparison table; its
  "Take" column names only Euclidean rhythms as a candidate. Claude
  does not have the earlier discussion.
- Why it matters: each function is new notation and new loader code.
- Options (any number):
  - Euclidean rhythms (Strudel `bd(3,8)`: 3 hits spread over 8 steps).
  - Repeat a step (Strudel `!`).
  - Play a group n times as fast (Strudel `*2`).
  - Weights (Strudel `@`: `a@3 b` is 3 to 1).
- Claude's reading of "allowed in `.song` files": the function text
  stays in the file and is worked out each time the song is loaded.
  Correct this if it is wrong.
- Used unless you say otherwise: nothing; this one needs your list.
- Raised: Claude, 11 Nov 2026.

  **Jim's answer C-Q23**
  - Repeated sections in music have different bar symbols
  - Repeated elements use another symbol
  - |%n ... %| would mean repeat the bars n times, if no n, then n=1
  - as a short hand we also have a need for a repeat prior symbol (not repeat bar(s): how bout .. .. )? any parsing issues?

---

**OPEN_QUESTION C-Q25:** How does a song state the beat grouping of a
meter such as 7/8 or 5/4?

- Refers to: **Jim's Response Q25**, item 3, line 991.
- Context: the question was not about rounding. A 7/8 bar may be grouped
  2+2+3 or 3+2+2, and a 5/4 bar 3+2 or 2+3. The grouping decides where
  the accents fall and how the notes are beamed. You said beat
  groupings must be handled.
- Why it matters: without it the staff and the beat pulse cannot show
  the grouping the composer means.
- Options:
  - (a) A setting on the section, e.g. `meter 7/8  grouping 2+2+3`,
    which sets accents and beaming only; the beat stays the eighth.
  - (b) The same setting, and the groups become the beats that the
    tempo and the counter count.
- Used unless you say otherwise: (a).
- Raised: Claude, 11 Nov 2026.

  **Jim's answer C-Q25**
  - YES to (a) and if grouping is ommited just means the beat and pulse are identical
