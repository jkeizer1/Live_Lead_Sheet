# Live Lead Sheet — Musical Time and Clock

Revision 11 — 29 Oct 2026

How the program represents *where* things are in a song, *how long* they
last, and *when* they happen. Every term is defined before it is used;
every number carries its unit; every choice gives its reason. Supersedes
the short form of D-7 in Architecture.md, which should point here.

**Rule: no math without units.** Every quantity in this document, in the
code (type and variable names) and in tests carries its unit.

Numbers are written without thousands separators (20160, not 20 160) so
that no digit grouping can be misread as an operator.

## 0. Units

| Symbol | Unit | Defined as |
|---|---|---|
| tick | tick | 1/5040 of a quarter note — the unit of score quantity |
| q | quarter note | 5040 ticks |
| w | whole note | 4 q = 20160 ticks |
| pulse | MIDI clock pulse | 1/24 of a quarter note = 210 ticks |
| count | one count of a bar | one whole note (w) divided by the time signature's lower number: in 4/4, 1/4 of a whole note = 5040 ticks; in 6/8, 1/8 of a whole note = 2520 ticks |
| min, s, ns | minute, second, nanosecond | 1 min = 60 s; 1 s = 1000000000 ns |
| mq | milli-quarter | 1/1000 of a quarter note — used only for tempo |
| mq/min | tempo | milli-quarters per minute; 120 BPM = 120000 mq/min |

**Writing units.** Ordinary English throughout, with one rule:

- **Unit words take the English plural**, exactly as in "5 metres" or
  "a speed in metres per second": *5040 ticks*, *24 pulses*, *6 counts*,
  *1 tick*. That applies everywhere, including table headings.
- **In a rate, the word after "per" (or after /) stays singular**, as in
  "metres per second": *5040 ticks/q*, read "5040 ticks per quarter
  note".
- **Abbreviations are never pluralized**, as with *s*, *km* or *kg* in
  everyday English: *60 s*, *5 ns*, *4 q*, *120000 mq/min*.

**Symbols and their units.** Formulas use the symbols below. Each
symbol's unit is stated here once, as is usual in technical writing;
formulas then show the symbols, constants carry their units, and a unit
check after each formula shows how the units cancel.

| Symbol | Meaning | Unit |
|---|---|---|
| P, P₀ | score position; P₀ is an anchor's position | ticks |
| ΔP | score length, P − P₀ | ticks |
| T, T₀ | instant; T₀ is an anchor's instant | ns |
| B | tempo | mq/min |
| n | upper number of the time signature: counts per bar | counts |
| d | lower number of the time signature: counts per whole note | counts/w |
| L_count | length of one count | ticks/count |
| L_bar | length of one bar | ticks |

**Counting units.** *count*, *pulse* and *beat* are counting units —
dimensionless in the strict physical sense, like "items". They are kept
in unit checks anyway, so that cancellations stay visible: for example
counts × ticks/count → ticks.

**Products are written with ×.** A space separates a number from its
unit (*5040 ticks*, *60 s*), as in ordinary measurement writing. Where a
value is itself an expression — a fraction or a variable — the
multiplication is written out with × or in words, never left to a space:
*(1/d) × w*, or *1/d of a whole note*; never *1/d w*.

Conversion factors used below, each with its unit:

| Factor | Value |
|---|---|
| ticks per quarter | 5040 ticks/q |
| ticks per whole note | 20160 ticks/w |
| ticks per pulse | 210 ticks/pulse |
| milli-quarters per quarter | 1000 mq/q |
| nanoseconds per minute | 60000000000 ns/min |

## 1. Two kinds of quantity

A song has two different kinds of quantity, and the design keeps them
apart:

| Kind | Unit | Depends on tempo? | Example |
|---|---|---|---|
| **Score quantity** — where something is written, or how long it is written to last | tick | no | "bar 3 starts at 40320 ticks"; "a dotted eighth lasts 3780 ticks" |
| **Clock quantity** — when something happens, or how long it takes | ns | — | "bar 3 starts at 14000000000 ns (14.000 s)" |

Score quantities are fixed when the song is written. Clock quantities
exist only while the song plays, and are computed from score quantities
using the tempo. **Tempo (mq/min) is the conversion rate between the
two.**

Analogy: score quantities are distances marked along a road; tempo is a
speed; clock quantities are travel times. The markers on the road do not
move when the speed changes.

## 2. Definitions

**Quarter note (q).** The reference note value. Tempo, ticks and MIDI
clock are all defined against it, as in MIDI.

**Tick.** The unit of score quantity: 1 tick = 1/5040 of a quarter note. A tick is a
unit of *note length*, not of seconds. Nothing in the program "ticks" at
a rate; a tick is a unit of measure, like a millimetre on a ruler.

| Note value | Calculation | Length |
|---|---|---|
| Whole note | 4 q × 5040 ticks/q | 20160 ticks |
| Half note | 2 q × 5040 ticks/q | 10080 ticks |
| Quarter note | 1 q × 5040 ticks/q | 5040 ticks |
| Eighth note | 5040 ticks ÷ 2 | 2520 ticks |
| Sixteenth note | 5040 ticks ÷ 4 | 1260 ticks |
| Thirty-second note | 5040 ticks ÷ 8 | 630 ticks |
| Sixty-fourth note | 5040 ticks ÷ 16 | 315 ticks |
| Dotted quarter (3 eighths) | 3 × 2520 ticks | 7560 ticks |
| Dotted eighth (3 sixteenths) | 3 × 1260 ticks | 3780 ticks |
| Eighth-note triplet (a quarter in 3) | 5040 ticks ÷ 3 | 1680 ticks |
| Quarter-note triplet (a half in 3) | 10080 ticks ÷ 3 | 3360 ticks |

**Score position (P).** A whole number of ticks measured from
**P = 0 ticks, the first downbeat of the song.** This origin is fixed —
decided by Jim, 29 Oct 2026. Nothing moves it: "play from" a section
starts playing at that section's own position (not at 0 ticks), the
practice loop replays the same positions, and stop and restart change
nothing. Every bar, chord and trigger therefore has one position for the
life of the song. A position is never negative. (A count-in, EFS-2.7,
lies before 0 ticks and is handled by the player as its own lead-in; it
has no score positions.)

**Performance order.** Positions count along the song *as it is played*,
with repeats written out: a repeat is shorthand in the written song, not
a reason to reset position. If section A repeats twice, its second pass
has its own, later positions. The written song (section, repeat number,
bar, offset within the bar) is converted to performance positions once,
when playing starts. The strip shows bars in performance order
(EFS-1.6), so screen and positions agree.

**Score length (ΔP).** Ticks between two positions:
`ΔP = later P − earlier P`.

**Count length and bar length.** For time signature *n/d* — *n* counts
per bar, each count 1/*d* of a whole note:

```
L_count = 20160 ticks/w ÷ d
L_bar   = n × L_count
```

Unit checks: `ticks/w ÷ counts/w` → ticks/count; `counts × ticks/count`
→ ticks.

| Time signature | Count length | Bar length |
|---|---|---|
| 4/4 | 20160 ticks ÷ 4 = 5040 ticks | 4 × 5040 ticks = 20160 ticks |
| 3/4 | 5040 ticks | 3 × 5040 ticks = 15120 ticks |
| 6/8 | 20160 ticks ÷ 8 = 2520 ticks | 6 × 2520 ticks = 15120 ticks |
| 5/4 | 5040 ticks | 5 × 5040 ticks = 25200 ticks |
| 7/8 | 2520 ticks | 7 × 2520 ticks = 17640 ticks |
| 2/2 (cut time) | 20160 ticks ÷ 2 = 10080 ticks | 2 × 10080 ticks = 20160 ticks |

Count 1 of each bar is the bar's first position (EFS-1.3).

**Instant (T).** A clock reading: a whole number of nanoseconds from
**the clock** (section 5). Instants are only compared and subtracted;
their absolute value has no meaning. `ΔT` is a duration.

**Tempo (B).** Quarter notes per minute, stored as a whole number of
milli-quarters per minute: `B = BPM × 1000 mq/q`.
120 BPM is stored as 120000 mq/min. The 1000 allows slider settings such
as 112.44 BPM (112440 mq/min) while keeping all arithmetic in whole
numbers.

**Anchor.** A known correspondence between the two kinds of quantity:

```
anchor = (P₀, T₀, B)
```

meaning: *position P₀ sounded at instant T₀, and from there on the tempo
is B*. An anchor is the only place where score and clock meet.

## 3. Why ticks are counted per quarter note, not per second

A per-second unit makes note lengths depend on tempo. With
576000 ticks/s, a quarter note lasts
`576000 ticks/s × 60 s/min ÷ BPM` ticks:

| Tempo | Quarter note length at 576000 ticks/s |
|---|---|
| 120 q/min | 34560000 ticks/min ÷ 120 q/min = 288000 ticks/q — whole |
| 137 q/min | 34560000 ticks/min ÷ 137 q/min = 252262.77… ticks/q — not whole |
| 140 q/min | 34560000 ticks/min ÷ 140 q/min = 246857.14… ticks/q — not whole |

At most tempos — and at almost every slider setting — written notes
would fall between ticks and need rounding, and rounding errors added
bar after bar become drift. A per-quarter unit has no such problem: a
dotted eighth is 3780 ticks at every tempo. Tempo enters only when a
position is converted to an instant (section 6).

## 4. Why 5040 ticks/q

The tick count per quarter must divide exactly by every subdivision the
music uses. Each prime factor of the count permits one kind of division:

```
5040 = 2⁴ × 3² × 5 × 7 = 16 × 9 × 5 × 7
```

| Factor | Makes exact | Needed for |
|---|---|---|
| 2⁴ = 16 | 5040 ticks ÷ 2, 4, 8, 16 | eighths, sixteenths, 32nds, 64ths |
| 3 | 5040 ticks ÷ 3 | triplets, swing |
| 3² = 9 | 5040 ticks ÷ 9 = 560 ticks | a triplet inside a triplet |
| 5 | 5040 ticks ÷ 5 = 1008 ticks | quintuplets (rare) |
| 7 | 5040 ticks ÷ 7 = 720 ticks | septuplets (rare) |

**Time signatures need none of these factors.** The upper number
*multiplies* (5/4: 5 counts × 5040 ticks/count); the lower number divides
a whole note by 2, 4, 8 or 16 only, which 2⁴ covers. Dotted values
multiply too (a dotted quarter = 3 × 2520 ticks). So 3/8, 6/8, 12/8, 5/4
and 7/8 are all exact with factor 2 alone.

**MIDI clock** sends 24 pulses/q:
`5040 ticks/q ÷ 24 pulses/q = 210 ticks/pulse`, exactly.

**Overflow.** Positions are stored in 64 bits (largest value
18446744073709551615 ticks). At 300 q/min the score advances
`300 q/min × 5040 ticks/q ÷ 60 s/min = 25200 ticks/s`, so a counter
lasts `18446744073709551615 ticks ÷ 25200 ticks/s ≈ 7.3 × 10¹⁴ s`, about
23 million years.

**Alternatives.** Any count containing the needed factors works:
144 ticks/q (2⁴ × 3²) covers everything except quintuplets and
septuplets; 720 ticks/q adds quintuplets; 5040 ticks/q adds septuplets at
no cost. For comparison, Pro Tools counts 960 ticks/q in its display and
960000 ticks/q internally
([Avid](https://apps.avid.com/protoolsfirsthelp/version12.3/enu/Pro%20Tools%20First%20Help/Arr1.conductors.31.09.html));
960 = 2⁶ × 3 × 5 cannot divide a quarter by 9 or 7.

*Open for Jim:* keep 5040 ticks/q, or choose 144 or 720 so the number
contains only factors actually used.

## 5. The clock

There is **one clock** for the whole program: a steady counter of real
time in nanoseconds that never changes speed and never jumps. Tempo
never changes the clock; tempo only changes how positions are converted
to instants.

Which counter that is — Linux's monotonic clock, or the ALSA sequencer
queue's own real-time count — is decision D-8 (pending). Section 8
explains why only one may be used.

## 6. Converting between position and instant

Given an anchor (P₀, T₀, B):

**Position → instant** (when should this event be sent?):

```
T = T₀ + (P − P₀) × 60000000000 ns/min × 1000 mq/q ÷ (B × 5040 ticks/q)
```

Unit check — cancel each unit:
`ticks × ns/min × mq/q ÷ (mq/min × ticks/q)` → ticks cancel ticks,
mq cancels mq, q cancels q, min cancels min → **ns**.

Reading it step by step:

1. `(P − P₀) ÷ 5040 ticks/q` → quarter notes.
2. `÷ (B ÷ 1000 mq/q)` → minutes.
3. `× 60000000000 ns/min` → nanoseconds.

The code multiplies everything first and divides once, so every
intermediate value is a whole number. The two constants combine to
`60000000000 ns/min × 1000 mq/q = 60000000000000 ns·mq/(min·q)`.

**Instant → position** (where is the playhead now?):

```
P = P₀ + (T − T₀) × B × 5040 ticks/q ÷ (60000000000 ns/min × 1000 mq/q)
```

Unit check: `ns × mq/min × ticks/q ÷ (ns/min × mq/q)` → **ticks**.

**Arithmetic rules.**

1. Every conversion starts from an anchor. No instant is ever computed
   by adding the previous event's duration to the previous instant, so
   no error can accumulate.
2. The multiplication is done in 128-bit integers before the single
   division, so nothing overflows. Worst case, a day-long song at
   300 q/min: `P − P₀ = 25200 ticks/s × 86400 s = 2177280000 ticks`;
   `2177280000 ticks × 60000000000000 ns·mq/(min·q) ≈ 1.3 × 10²⁶` as a
   whole number, far below the 128-bit limit of 3.4 × 10³⁸.
3. Position → instant rounds to the nearest nanosecond; instant →
   position rounds down (the playhead never shows a position not yet
   reached). Each rounding happens once, inside one calculation, so the
   error is at most 0.5 ns and is never carried forward.

## 7. Worked examples

All examples start the song at T₀ = 10000000000 ns (10.000 s), so the
first anchor is (P₀ = 0 ticks, T₀ = 10000000000 ns, B = 120000 mq/min).
Let K = 60000000000 ns/min × 1000 mq/q = 60000000000000 ns·mq/(min·q).

**Example 1 — 4/4 at 120 q/min.** Bar 2 starts at P = 20160 ticks.

```
T = 10000000000 ns + (20160 ticks − 0 ticks) × K ÷ (120000 mq/min × 5040 ticks/q)
  = 10000000000 ns + 1209600000000000000 ticks·ns·mq/(min·q)
                     ÷ 604800000 mq·ticks/(min·q)
  = 10000000000 ns + 2000000000 ns
  = 12000000000 ns  (12.000 s)
```

A 4/4 bar at 120 q/min lasts 2000000000 ns (2 s): 4 q × 0.5 s/q.

**Example 2 — a tempo change.** The next section starts at bar 3,
P = 40320 ticks, and is marked 140 q/min. By example 1's formula bar 3
starts at 14000000000 ns, so a new anchor is made there:
(P₀ = 40320 ticks, T₀ = 14000000000 ns, B = 140000 mq/min).
Bar 4 starts at P = 60480 ticks:

```
T = 14000000000 ns + (60480 ticks − 40320 ticks) × K ÷ (140000 mq/min × 5040 ticks/q)
  = 14000000000 ns + 1209600000000000000 ticks·ns·mq/(min·q)
                     ÷ 705600000 mq·ticks/(min·q)
  = 14000000000 ns + 1714285714.29… ns
  → 15714285714 ns  (15.714285714 s; rounded once, to the nearest ns)
```

**Example 3 — 6/8.** At 120 q/min a quarter lasts 0.5 s. A 6/8 bar is
15120 ticks = 15120 ticks ÷ 5040 ticks/q = 3 q = 1.5 s. Its two felt beats
are dotted quarters, 7560 ticks = 1.5 q = 0.75 s each; its six counts are
eighths, 2520 ticks = 0.5 q = 0.25 s each.

**Example 4 — the tempo slider at 90 %.** 120 q/min × 0.90 = 108 q/min
= 108000 mq/min. A new anchor is made where the slider takes effect.
One 4/4 bar then lasts

```
20160 ticks × K ÷ (108000 mq/min × 5040 ticks/q) = 2222222222 ns  (≈ 2.222 s)
```

**Example 5 — MIDI clock pulses.** A pulse is sent at every multiple of
210 ticks. At 120 q/min the interval between pulses is

```
210 ticks × K ÷ (120000 mq/min × 5040 ticks/q) = 20833333 ns  (≈ 20.833 ms)
```

which is 0.5 s/q ÷ 24 pulses/q. Each pulse's instant is computed from
the anchor, never by adding 20833333 ns repeatedly.

## 8. Anchors in practice

A new anchor is made at:

- **Play** — (start position, the instant playing starts,
  written tempo × slider).
- **A section with a new written tempo** — at its first position, using
  the instant computed for that position from the previous anchor.
- **A slider change** — at the next position not yet handed to the MIDI
  output (D-8 proposes handing events to ALSA up to about 50 ms ahead;
  the slider then takes effect within 50 ms).
- **Play from a section** — as for Play, with the start position being
  that section's first position.
- **Practice loop** — each time the loop goes back to the start of its
  section: (that section's first position, the instant of the jump,
  current tempo). Playing goes back; the positions themselves are
  unchanged.
- **Loop release, stop and restart** — as for Play.

**Why one clock only.** If positions were converted using one clock and
the MIDI output delivered events by another, any offset or rate
difference between the two would appear as timing error. Both the
program and the ALSA queue keep their own real-time count from their own
starting moment. Using either one as *the* clock, and never mixing them,
removes the problem (D-8).

## 9. Displayed tempo versus internal tempo

**Decided (Jim, 29 Oct 2026):** the tempo the musician enters and sees,
in BPM, counts the time signature's **counts** per minute — the note
value of its lower number. In 4/4, 120 BPM is 120 quarter notes per
minute; in 3/8 or 6/8, 120 BPM is 120 eighth notes per minute; in 2/2,
60 BPM is 60 half notes per minute. This matches how bars are counted
(EFS-1.3: 6/8 counts 1–6).

Internally, tempo is always B, in mq/min (section 2). The conversion
uses the count length L_count:

```
quarter tempo = displayed tempo × L_count ÷ 5040 ticks/q
B             = quarter tempo × 1000 mq/q
```

Unit check: `counts/min × ticks/count ÷ ticks/q` → q/min;
`q/min × mq/q` → mq/min.

| Time signature | Displayed tempo | L_count | Quarter tempo | B |
|---|---|---|---|---|
| 4/4 | 120 BPM | 5040 ticks/count | 120 counts/min × 5040 ticks/count ÷ 5040 ticks/q = 120 q/min | 120000 mq/min |
| 3/8 | 120 BPM | 2520 ticks/count | 120 counts/min × 2520 ticks/count ÷ 5040 ticks/q = 60 q/min | 60000 mq/min |
| 6/8 | 120 BPM | 2520 ticks/count | 60 q/min | 60000 mq/min |
| 2/2 | 60 BPM | 10080 ticks/count | 60 counts/min × 10080 ticks/count ÷ 5040 ticks/q = 120 q/min | 120000 mq/min |

**Consequence to keep in mind:** when a song moves between meters with
different lower numbers, the same displayed number means a different
speed. A 4/4 section at 120 BPM followed by a 6/8 section at 120 BPM
plays the eighth notes of the 6/8 at the speed of the quarter notes of
the 4/4 — the music halves in quarter-note speed. The song file states
each section's tempo in its own counts, so this is always explicit.

## 10. Names in the code

- **Units in names.** Every quantity's name or type states its unit:
  `TICKS_PER_QUARTER_NOTE`, `NANOSECONDS_PER_MINUTE`, a tempo field
  `milli_quarters_per_minute`, an instant `instant_ns`. A bare number
  never stands for a quantity.
- **Decided (Jim, 29 Oct 2026):** the code type `MusicalTime` is renamed
  `ScorePosition` everywhere, since "time" invited the misreading that a
  tick has a duration in seconds.
- **Decided (Jim, 29 Oct 2026): two types.** `ScorePosition` is a place
  in the song (P); `ScoreLength` is a distance along the song (ΔP). Both
  are counted in ticks; they differ in role. Allowed operations, all
  checked so overflow is reported and never wraps:
  - `ScorePosition − ScorePosition → ScoreLength` (an error if the
    second is later; lengths are forward-only, never negative)
  - `ScorePosition + ScoreLength → ScorePosition`
  - `ScoreLength + ScoreLength → ScoreLength`
  - whole number × `ScoreLength` → `ScoreLength` (e.g. 4 × one bar)

  `ScorePosition + ScorePosition` is not defined, so the compiler
  refuses it; so is passing a length where a position is expected.
- **One base unit.** Every score quantity is held in ticks. Note values
  (a quarter, a dotted eighth) are converted to ticks when they are
  created, so quantities in different note units are never mixed as raw
  numbers.
- "Tick" means only the unit of 1/5040 of a quarter note. MIDI's clock messages, sent at
  24 pulses/q, are always **clock pulses**.
- "Instant" means a clock reading in ns; "duration" means a difference
  of instants.

## 11. Open decisions

1. Keep 5040 ticks/q, or 144 / 720 ticks/q (section 4).
2. Which counter is the clock, ALSA timer resolution, and the lead
   window — D-8 in Architecture.md.
