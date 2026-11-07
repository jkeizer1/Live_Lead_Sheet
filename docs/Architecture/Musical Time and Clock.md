# Live Lead Sheet — Musical Time and Clock

Revision 17 — 7 Nov 2026

How the program represents *where* things are in a song, *how long* they
last, and *when* they happen. Every term is defined before it is used;
every number carries its unit; every choice gives its reason. This is
the full reference for decisions D-7 and D-8; Architecture.md holds
short summaries that point here.

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
| clock pulse | MIDI clock pulse | 1/24 of a quarter note = 210 ticks |
| pulse | one pulse of a bar | the note value of the time signature's lower figure: a whole note (w) divided by it. In 4/4, 1/4 of a whole note = 5040 ticks; in 6/8, 1/8 of a whole note = 2520 ticks |
| beat | one beat of a bar | in simple time 1 pulse; in compound time (upper figure 6, 9 or 12) 3 pulses, a dotted note (section 2) |
| min, s, ms, µs, ns | minute, second, millisecond, microsecond, nanosecond | 1 min = 60 s; 1 s = 1000 ms = 1000000 µs = 1000000000 ns |
| mq | milli-quarter | 1/1000 of a quarter note — used only for tempo |
| mq/min | tempo, inside the program | milli-quarters per minute; 120 q/min = 120000 mq/min |

**Writing units.** Ordinary English throughout, with one rule:

- **Unit words take the English plural**, exactly as in "5 metres" or
  "a speed in metres per second": *5040 ticks*, *24 clock pulses*, *6 pulses*,
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
| n | upper figure of the time signature: pulses per bar | pulses |
| d | lower figure of the time signature: pulses per whole note | pulses/w |
| L_pulse | length of one pulse | ticks/pulse |
| L_beat | length of one beat | ticks/beat |
| L_bar | length of one bar | ticks |

**Counting units.** *pulse*, *beat* and *clock pulse* are counting units —
dimensionless in the strict physical sense, like "items". They are kept
in unit checks anyway, so that cancellations stay visible: for example
pulses × ticks/pulse → ticks.

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
| ticks per clock pulse | 210 ticks/clock pulse |
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
nothing. Every bar, chord and phrase cue therefore has one position for the
life of the song. A position is never negative. A count-in is a section
of the score (D-10, EFS-2.7), so P = 0 ticks is the first downbeat of the
count-in when the song has one; its bars have positions like any other.

**Performance order.** Positions count along the song *as it is played*,
with repeats written out: a repeat is shorthand in the written song, not
a reason to reset position. If section A repeats twice, its second pass
has its own, later positions. The written song (section, repeat number,
bar, offset within the bar) is converted to performance positions once,
when playing starts. The strip shows bars in performance order
(EFS-1.6), so screen and positions agree.

**Score length (ΔP).** Ticks between two positions:
`ΔP = later P − earlier P`.

**Pulse, beat and bar.** *Decided (Jim, 7 Nov 2026), following the
rudiments (Barbara Wharram, Elementary Rudiments of Music, ch. 7, "Time").*
For time signature *n/d*, the **pulse** is the lower figure's note value,
1/*d* of a whole note, and a bar has *n* pulses. The **beat** depends on
the kind of time:

- **Simple time** (every *n* other than 6, 9 or 12): a beat is one pulse;
  *n* beats a bar.
- **Compound time** (*n* = 6, 9 or 12: duple, triple, quadruple): a beat
  is three pulses, a dotted note; *n* ÷ 3 beats a bar. No lower figure
  can name a dotted note, which is why the upper figure counts pulses.

```
L_pulse = 20160 ticks/w ÷ d
L_beat  = L_pulse                (simple time)
L_beat  = 3 pulses × L_pulse     (compound time)
L_bar   = n × L_pulse
```

Unit checks: `ticks/w ÷ pulses/w` → ticks/pulse;
`pulses × ticks/pulse` → ticks.

| Time signature | Kind | Pulse | Beats a bar | Beat | Bar |
|---|---|---|---|---|---|
| 4/4 | simple quadruple | 20160 ticks ÷ 4 = 5040 ticks | 4 | 5040 ticks | 4 × 5040 ticks = 20160 ticks |
| 3/4 | simple triple | 5040 ticks | 3 | 5040 ticks | 15120 ticks |
| 3/8 | simple triple | 2520 ticks | 3 | 2520 ticks | 7560 ticks |
| 2/2 (cut time) | simple duple | 10080 ticks | 2 | 10080 ticks | 20160 ticks |
| 5/4 | simple quintuple | 5040 ticks | 5 | 5040 ticks | 25200 ticks |
| 7/8 | simple septuple | 2520 ticks | 7 | 2520 ticks | 17640 ticks |
| 6/8 | compound duple | 2520 ticks | 6 ÷ 3 = 2 | 3 × 2520 ticks = 7560 ticks | 15120 ticks |
| 9/8 | compound triple | 2520 ticks | 3 | 7560 ticks | 22680 ticks |
| 12/8 | compound quadruple | 2520 ticks | 4 | 7560 ticks | 30240 ticks |
| 9/4 | compound triple | 5040 ticks | 3 | 15120 ticks | 45360 ticks |
| 12/16 | compound quadruple | 1260 ticks | 4 | 3780 ticks | 15120 ticks |

Check: beats a bar × beat = bar (6/8: 2 beats × 7560 ticks/beat = 15120
ticks = 6 pulses × 2520 ticks/pulse).

Beat 1 of each bar is the bar's first position (EFS-1.3). The felt
grouping of an irregular meter (5/8 as 2+3, 7/8 as 2+2+3) is its **beat
grouping**, a matter of display; its beat for tempo stays the pulse.
*Open:* an upper figure that is a multiple of 3 outside the rudiments
(15/8, 18/8) is refused when a song is read, with a message; and a
meter meant against its figures (a 9/8 as 2+2+2+3, a 6/4 in three half
notes) needs an override, decided with beat grouping.

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
*multiplies* (5/4: 5 pulses × 5040 ticks/pulse); the lower number divides
a whole note by 2, 4, 8 or 16 only, which 2⁴ covers. Dotted values
multiply too (a dotted quarter = 3 × 2520 ticks). So 3/8, 6/8, 12/8, 5/4
and 7/8 are all exact with factor 2 alone.

**MIDI clock** sends 24 clock pulses/q:
`5040 ticks/q ÷ 24 clock pulses/q = 210 ticks/clock pulse`, exactly.

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

**Decided (Jim, 29 Oct 2026): 5040 ticks/q.** It covers every
subdivision above, septuplets included, at no extra cost over 144 or
720 ticks/q.

## 5. The clock

There is **one clock** for the whole program: a steady counter of real
time in nanoseconds that never changes speed and never jumps. Tempo
never changes the clock; tempo only changes how positions are converted
to instants.

**Decided (Jim, 29 Oct 2026, D-8):** the clock is **the ALSA sequencer
queue's own real time** — the real-time count of the one queue the
program creates for its MIDI output.

- **Who reads it.** The player reads it for every conversion between
  position and instant (section 6). The screen reads it once per frame
  to place the playhead. No other counter of real time is used for
  music.
- **Who supplies it.** The ALSA output adapter. The player and the
  screen ask the adapter for the current instant; nothing outside the
  adapter knows it comes from ALSA, so platform code stays in the
  adapter.
- **The queue's timer.** The program does not select a timer. A new
  queue gets the ALSA high-resolution timer (HR timer) by default on
  Jim's machine (evidence below). After creating the queue, the ALSA
  output adapter reads `/proc/asound/seq/timer`, finds the line
  "Timer for queue N : ..." for its own queue number N, and checks that
  the timer is named "HR timer". If the line is missing, unreadable, or
  names another timer, this is reported in the warnings region (A-8,
  EFS-1.14); the program keeps running and never hides it.
  The check catches the slow system timer (events about ±2 ms off), but
  it does **not** guarantee fine delivery: with the HR timer the queue
  still releases events on a 1 ms grid (measurements below).
  *Explicit selection is not needed.* It would need the C calls
  `snd_seq_queue_timer_set_type` and `snd_seq_set_queue_timer`, which the
  `alsa` crate (0.12.1) does not offer, and it keeps its `snd_seq_t`
  handle private, so a separate FFI binding cannot reach our queue. It
  was also tried (the Linux MIDI Router's spike 1 clock source with
  `--high-resolution-timer`, 29 Oct 2026): clock pulse intervals at 120 q/min
  were 19.999 to 21.024 ms with it, against 19.998 to 21.001 ms from our
  queue's default, so it changes nothing on this machine.
- **How events are stamped.** Every event handed to the queue carries
  its instant, in real time, computed from the current anchor
  (section 6). The queue has no tempo of its own: tempo exists only in
  the anchor.
- **Lead window: 50 ms.** Events are handed to the queue up to 50 ms
  before their instant. The kernel then delivers each one **between 0
  and 1 ms after its instant**, on the queue's 1 ms grid, whatever the
  program's own threads are doing — for example while the screen is
  drawing. Section 8 says what happens to events already queued when the
  music changes.
- **Fallback, tested and not adopted.** A thread that sleeps until each
  instant and sends the event at once was measured against the queue
  (below). It is ten times more exact for 99% of clock pulses but its worst
  case is worse than the queue's under heavy load, so the queue stays.

**Evidence** (Jim's machine, 29 Oct 2026, from `/proc/asound/timers`,
`/proc/asound/seq/timer` and `lsmod`):

| Item | Found |
|---|---|
| HR timer | present; resolution 0.001 µs (1 ns) |
| `snd_hrtimer` module | loaded, used by `snd_timer` |
| Default for an existing queue | already the HR timer |
| System timer | resolution 1000 µs (1 ms) |
| Queue skew | 65536/65536 = 1.0 (the queue runs at its timer's own rate) |

Resolution is the smallest step the timer can count, not how accurately
events arrive. Delivery accuracy was measured as described next.

**Delivery measured** (Jim's machine, 29 Oct 2026; examples
`delivery_spread` and `deadline_spread` in the `alsa_output` crate,
release build, 3000 clock pulses per row). A second ALSA client of the
same program receives the clock pulses; the kernel stamps each arrival with
the queue's real time. *Delay* is arrival instant minus the instant the
clock pulse was meant for. *Load* is busy threads competing for the 24 hardware
threads of the CPU.

| Sender | Tempo, load | Median | p99 | Worst | Jitter (p99 − min) |
|---|---|---|---|---|---|
| Queue (adopted) | 120 q/min, none | 0.626 ms | 0.960 ms | 0.961 ms | 0.833 ms |
| Queue | 240 q/min, 16 threads | 0.515 ms | 0.932 ms | 0.932 ms | 0.916 ms |
| Queue | 240 q/min, 32 threads | 0.580 ms | 0.997 ms | 0.997 ms | 0.916 ms |
| Sleep to deadline | 120 q/min, none | 0.058 ms | 0.062 ms | 0.871 ms | 0.035 ms |
| Sleep to deadline | 240 q/min, 32 threads | 0.057 ms | 0.062 ms | 2.973 ms | 0.006 ms |
| Sleep, then busy-wait last 200 µs | 120 q/min, none | 0.001 ms | 0.003 ms | 1.805 ms | 0.002 ms |
| Sleep, then busy-wait last 200 µs | 240 q/min, 32 threads | 0.001 ms | 2.652 ms | 6.321 ms | 2.651 ms |

What this shows:

- The queue releases every clock pulse within 1 ms after its instant, with or
  without load. Its average lateness (0.5 ms) is harmless: everything is
  late together. Clock pulses arrive on a 1 ms grid (intervals of 9.999 to
  11.000 ms around the ideal 10.417 ms at 240 q/min).
- Sleeping to deadlines is typically about ten times more exact, but under
  oversubscription about 1 clock pulse in 1000 arrives up to 3 ms late, and even
  idle one clock pulse was 0.87 ms late. Busy-waiting is exact on a free CPU
  and worse than the queue when the CPU is oversubscribed (the waiting
  thread is descheduled).
- 32 busy threads on 24 hardware threads is harsher than the real program.
  3000 clock pulses give only 3 samples at p99.9, so tail figures are rough.

**Is it audible?** The adopted queue's worst case, 1 ms, is far below
what listeners are generally reported to notice. From memory of the
timing-perception literature, not checked for this document: a trained
listener detects a timing deviation in a steady rhythm only from about
2 to 5 ms, and a hit that is early or late by under 10 ms is heard as
feel, not as an error. The followers' own timing is coarser still: USB
MIDI delivers in 1 ms frames, and a MIDI cable takes 0.32 ms per byte.
Even the worst case measured for any sender, 6.3 ms for a single clock pulse in
the harshest run, is at the edge of what a trained listener notices, and
only if the follower steps on every clock pulse. So the choice between senders
rests on predictability, not on audibility. The listening test (Part B of
the Linux MIDI Router's spike 1) has not been done for this program; it
remains the final check, with the real drawing load, through the
followers Jim uses.

Section 8 explains why only one clock may be used.

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
15120 ticks = 15120 ticks ÷ 5040 ticks/q = 3 q = 1.5 s. Its two beats
are dotted quarters, 7560 ticks = 1.5 q = 0.75 s each; its six pulses
are eighths, 2520 ticks = 0.5 q = 0.25 s each.

**Example 4 — the tempo slider at 90 %.** 120 q/min × 0.90 = 108 q/min
= 108000 mq/min. A new anchor is made where the slider takes effect.
One 4/4 bar then lasts

```
20160 ticks × K ÷ (108000 mq/min × 5040 ticks/q) = 2222222222 ns  (≈ 2.222 s)
```

**Example 5 — MIDI clock pulses.** A clock pulse is sent at every multiple of
210 ticks. At 120 q/min the interval between clock pulses is

```
210 ticks × K ÷ (120000 mq/min × 5040 ticks/q) = 20833333 ns  (≈ 20.833 ms)
```

which is 0.5 s/q ÷ 24 clock pulses/q. Each clock pulse's instant is computed from
the anchor, never by adding 20833333 ns repeatedly.

## 8. Anchors in practice

The player hands events to the queue in position order, up to 50 ms
ahead (section 5). **A change in the music takes effect at the end of
what is already queued** — the first position not yet handed to the
queue. Queued events are removed only where a case below says so.

A new anchor is made at:

- **Play** — (start position, the instant playing starts,
  written tempo × slider).
- **A section with a new written tempo** — at its first position, using
  the instant computed for that position from the previous anchor.
  Nothing at or after that position has been queued yet, so nothing is
  removed.
- **A slider change** — at the first position not yet queued, using the
  instant computed for that position from the previous anchor. Nothing
  is removed. Since at most 50 ms of events are queued, the slider takes
  effect within 50 ms.
- **Play from a section** — as for Play, with the start position being
  that section's first position.
- **Practice with *count-in first*** (EFS-2.8) — as for Play at
  P = 0 ticks; at the end of the count-in section, a jump as for the
  practice loop: (the section's first position, the instant of the jump,
  current tempo).
- **Practice loop** — each time the loop goes back to the start of its
  section: (that section's first position, the instant of the jump,
  current tempo). Playing goes back; the positions themselves are
  unchanged.
- **Restart after stop** — as for Play.

**Loop release** (EFS-2.8) means "carry on into the next section", so
playing never stops and **MIDI Stop is never sent**. Like a tempo
change, the release takes effect at the first position not yet queued:

- *The jump back has not been queued yet* — almost always, since the
  jump is queued only within 50 ms of the section's end. The player
  carries on into the next section. Nothing is removed.
- *The jump back has already been queued* — the release came within the
  last 50 ms before the jump. The queued events are removed, and a
  Note Off is sent directly for exactly the notes the player knows are
  sounding. All Notes Off is **not** sent: it would also cut notes the
  musician is playing on the same channel. A new anchor is made at the
  next section's first position, at the instant the jump would have
  happened, with the current tempo.

**Stop** (EFS-2.1), in this order:

1. Remove all queued events.
2. Send a Note Off directly for each note the player knows is sounding.
3. Send MIDI Stop.
4. Send All Notes Off on every port in use, as a final safety net.

**Why one clock only.** If positions were converted using one clock and
the MIDI output delivered events by another, any offset or rate
difference between the two would appear as timing error. Linux's
monotonic clock and the ALSA queue each keep their own real-time count
from their own starting moment. Using the queue's real time as *the*
clock, for conversions, for delivery and for the playhead, removes the
problem (section 5).

## 9. Sending MIDI clock

The player owns the MIDI clock (EFS-2.5): it creates the clock and its
followers count it. This section gives the MIDI messages, what a clock
owner canonically does with them, and how the player does that with the
clock of section 5.

**The messages** (MIDI 1.0, System Real-Time and System Common):

| Message | Byte | What the owner means by it |
|---|---|---|
| Timing Clock | F8 | one clock pulse; 24 clock pulses/q |
| Start | FA | play from the song's beginning; the next Timing Clock is its first clock pulse |
| Stop | FC | stop; followers keep their position |
| Continue | FB | play from the followers' current position; the next Timing Clock is its first clock pulse |
| Song Position Pointer | F2 | while stopped: set the position, in MIDI beats |

A MIDI beat is a sixteenth note: 6 clock pulses × 210 ticks/clock pulse = 1260 ticks.
No message carries a tempo: tempo is the rate of Timing Clock.

**What a clock owner canonically does:**

- *Set the tempo:* send Timing Clock every 1 ÷ (24 clock pulses/q × tempo).
- *Start from the beginning:* Start, then the first Timing Clock at the
  downbeat.
- *Start from a position:* while stopped, Song Position Pointer with the
  position in MIDI beats, then Continue, then the first Timing Clock at
  that position. Followers may need a short gap after Song Position
  Pointer to locate (section 15, facts to verify).
- *Change tempo:* change the interval between clock pulses from the clock pulse where
  the change is written. Nothing else is sent.
- *While stopped:* many owners keep sending Timing Clock, so followers
  know the tempo before Start. The player does not (D-10): Timing Clock
  runs from Play to Stop, and the count-in section gives followers their
  clock pulses before the music's first bar.

**What a clock follower canonically does while stopped.** MIDI 1.0: after
Stop, a device synced to external clock ignores further Timing Clock;
Start or Continue takes effect at the next Timing Clock. So clock pulses sent
while stopped can set a tempo display but advance nothing.

**What the studio's devices were observed to do** (Linux MIDI Router
project, `docs/MIDI Connections/`: FH-2 Observations rev 3, Disting NT
Observations rev 1, NK Observations rev 3, SQ-1 Observations; tests of
1–2 Nov 2026):

| Device | Timing Clock while its transport is stopped | Start |
|---|---|---|
| FH-2 | tempo display measures the clock pulses; its **Clock** outputs give no clock pulses; the clock pulses are still forwarded to the disting NT | a **Start** (or Start/Cont) trigger output type gives a reset trigger |
| FH-2 gates driven by notes | — (notes, not clock) | stayed in step whatever the transport did |
| disting NT | MIDI monitor shows the clock pulses; Clock and Step Sequencer algorithms do not advance | Clock algorithm honours Start and Stop |
| NiftyKEYZ in external clock mode | its **clock jack keeps pulsing** in tempo, at a musical rate set by its division keys (rates not yet recorded) | its arp starts on Start |
| SQ-1, clock and transport on its `CTRL` port | not recorded | Stop then Start resets it to step 1 |

Consequences, written into D-10: Timing Clock alone cannot keep a
Eurorack divider running across a stop, through the FH-2 or the disting
NT; the **beat pulse** (EFS-2.6), a voice turned into a gate by
an FH-2 output, can, because notes do not depend on transport. Resetting
followers is done by phrase cues — Start on a follower's own output
port, or a reset note to an FH-2 trigger output.

**How the player does it.** Every message is stamped in real time from
the current anchor and handed to the ALSA sequencer queue up to 50 ms
ahead (section 5), like every other event:

- **Timing Clock** at every multiple of 210 ticks, its instant computed
  from the anchor (example 5), never by adding intervals.
- **Play** (EFS-2.1): the first Timing Clock at the instant of the start
  position, on the clock output only (EFS-2.5). Start, Stop and Continue
  are sent only by phrase cues (D-10, EFS-4.3) and by Stop (section 8).
  A Start cue at the same instant as a Timing Clock must be delivered
  first (section 15, facts to verify).
- **Play from a section:** as Play, at the section's first position.
  Song Position Pointer is not sent: many followers ignore it (D-10).
  The state in effect there is sent first (EFS-4.5).
- **Tempo changes** (written or slider): nothing extra is sent. The new
  anchor (section 8) changes the instants of the clock pulses not yet queued.

- **Stop:** the steps of section 8.

**Starting followers in step (D-10, 4 Nov 2026).** Clock and reset are
separate, as in Eurorack practice. A clock *divider* counts clock pulses, so
it follows from its next clock pulse; only its phase is unknown. A clock
*multiplier* measures the time between clock pulses, so it lags every tempo
change by about one input period: send the fastest rate anything needs
and divide down. (To check in the 4ms Quad Clock Divider's manual: that
its ÷N settings count and its ×N settings measure.) The count-in section
gives the dividers their clock pulses; a reset cue on the music's first bar
fixes their phase and starts the sequencers together. Everything uses
standard MIDI messages and notes, so setups without an FH-2 work the
same; the FH-2's Start trigger and note-driven gates are one way to carry
it into Eurorack.

## 10. Player states

| State | Meaning |
|---|---|
| Stopped | not playing; nothing queued (before the first Play, and after Stop) |
| Playing | Timing Clock and every track play as written |

Play moves Stopped to Playing; Stop moves Playing to Stopped. A count-in
is a section of the score, not a state (D-10). Each Play, play from a section, and loop jump makes an anchor
(section 8).

## 11. The screen keeps out of the MIDI path

The MIDI path has priority; the screen only follows (A-5).

- **The player never waits for the screen.** It publishes the song
  events (A-6: started, stopped, section, bar, beat, tempo, warnings) on
  a channel, and the screen takes all pending ones each frame. Commands
  from the screen (Play, Stop, play from, slider, loop release) travel
  the other way as typed messages on their own channel. A MIDI controller
  (EFS-9.1) sends the same typed commands, translated from its MIDI input,
  so remote control changes nothing in the player.
- **The screen draws each frame from the latest state, not from its own
  previous frame.** Once per frame it reads the queue's real time T on its
  own read-only handle (`queue_clock.rs`) and places the playhead at the
  position for T, from the anchor in effect (section 6, instant →
  position). How the screen gets the anchor in effect, when a new anchor
  has been made for a position not yet reached (a slider change, section
  8), is O-11.
- **Repair.** A late or dropped frame leaves nothing to catch up on: the
  next frame is computed from the latest state and is correct. The screen
  keeps no animation state that could drift.
- The screen never writes to the ALSA sequencer.

## 12. Following an external MIDI clock (future, not V1)

Not in V1: the requirement is EFS-2.9, marked later. Kept here so that
V1 does not block it.

**What a clock follower canonically does:**

- Counts Timing Clock; each clock pulse advances it 1/24 of a quarter note.
- On Start: goes to its start position; the next Timing Clock is the
  first clock pulse.
- On Continue: the next Timing Clock is the next clock pulse from where it
  stopped (or from a received Song Position Pointer).
- On Stop: stops sounding and keeps its position.
- Tempo is not received. It is measured from the interval between
  clock pulses, and is needed only for what falls between clock pulses.

While following, the screen's transport controls are disabled and the
player follows the clock owner's Start, Stop and Continue (EFS-2.9).
Where playing starts is still the player's own choice, set on the play
screen (EFS-2.1), for Start and Continue alike. The clock owner's
position 0 is the music's first bar, so the count-in section is not
played (EFS-2.9).

**How it could be done.** An ALSA sequencer queue cannot itself follow
an external MIDI clock, so the player counts clock pulses:

- The external source's port (e.g. the FH-2) is subscribed to the
  player's input port with timestamping on the player's queue in real
  time. Each received Timing Clock then carries the queue's real time of
  its arrival, set by the kernel, not by when the player's thread woke.
- A **clock pulse anchor** (k₀, P₀) says "clock pulse k₀ is position P₀":

  ```
  P = P₀ + (k − k₀) × 210 ticks/clock pulse
  ```

  Unit check: `clock pulses × ticks/clock pulse` → ticks.
- Events at Pₖ are sent at once; events after Pₖ and before Pₖ₊₁ are
  stamped in real time:

  ```
  T = Tₖ + (P − Pₖ) × I ÷ 210 ticks/clock pulse
  ```

  Unit check: `ticks × ns/clock pulse ÷ ticks/clock pulse` → ns.
- Displayed tempo:
  `B = 60000000000 ns/min × 1000 mq/q ÷ (24 clock pulses/q × I)`.
  Unit check: `ns/min × mq/q ÷ (clock pulses/q × ns/clock pulse)` → mq/min.
- **Song Position Pointer** while stopped: the play position becomes
  P_music + (MIDI beats received) × 1260 ticks/MIDI beat, where P_music
  is the position of the music's first bar (the length of the count-in
  section; 0 ticks without one).
- **Start or Continue:** the start position S is the music's first bar
  (P_music) or the play position, as the start control says. The clock pulse
  anchor is (0 clock pulses, S).
- **Clock lost:** if no Timing Clock arrives for 1 s while playing
  (EFS-2.9; a starting value, to be tested), the player stops, sends
  Note Off for every sounding note, and shows the message of EFS-2.9.
  It can wake for this check without a timer of its own: after each clock pulse
  it schedules an Echo event to its own input port at that clock pulse's
  arrival plus 1 s, and ignores the Echo if a later clock pulse came first.
- While the clock source is external, Stopped means waiting for Start or
  Continue from the clock owner.

| Symbol | Meaning | Unit |
|---|---|---|
| k, k₀ | clock pulse count since Start or Continue; k₀ at the clock pulse anchor | clock pulses |
| Pₖ, Tₖ | position and arrival instant of clock pulse k | ticks, ns |
| I | measured interval between clock pulses | ns/clock pulse |

Here the player sends notes as soon as its thread wakes, so its timing
depends on that thread. Following has priority over drawing (EFS-2.9);
whether that needs a real-time thread priority, which is a system
setting, is measured when following is spiked.

## 13. Tempo: beats per minute or per second

**Decided (Jim, 7 Nov 2026; replaces the 29 Oct decision, which counted
the lower figure's note value in every meter):** tempo counts **beats**
(section 2), per minute or per second. The song file writes
`tempo 30 beats/min` or `tempo 2 beats/s`; no note value is written, as
the meter already says what a beat is.

Inside, tempo is always B, in mq/min (section 2). The conversion uses the
beat length L_beat:

```
B = tempo in beats/min × L_beat × 1000 mq/q ÷ 5040 ticks/q
B = tempo in beats/s × 60 s/min × L_beat × 1000 mq/q ÷ 5040 ticks/q
```

Unit check: `beats/min × ticks/beat × mq/q ÷ ticks/q` → mq/min.

**Exact.** The shortest beat is a sixteenth (d = 16 is the largest lower
figure): 1260 ticks × 1000 mq/q ÷ 5040 ticks/q = 250 mq per beat. Every
beat is a whole multiple of a sixteenth, so a whole number of beats/min
or beats/s always gives a whole number of mq/min: B is exact, with no
rounding.

| Time signature | Tempo | L_beat | B |
|---|---|---|---|
| 4/4 | 120 beats/min | 5040 ticks/beat | 120 beats/min × 5040 ticks/beat × 1000 mq/q ÷ 5040 ticks/q = 120000 mq/min |
| 3/8 | 30 beats/min | 2520 ticks/beat | 15000 mq/min (15 q/min) |
| 2/2 | 60 beats/min | 10080 ticks/beat | 120000 mq/min |
| 5/4 | 100 beats/min | 5040 ticks/beat | 100000 mq/min |
| 6/8 | 60 beats/min | 7560 ticks/beat | 90000 mq/min (90 q/min, 180 eighths/min) |
| 9/8 | 60 beats/min | 7560 ticks/beat | 90000 mq/min |
| 12/8 | 60 beats/min | 7560 ticks/beat | 90000 mq/min |
| 9/16 | 80 beats/min | 3780 ticks/beat | 60000 mq/min |
| 6/8 | 2 beats/s | 7560 ticks/beat | 2 beats/s × 60 s/min × 7560 ticks/beat × 1000 mq/q ÷ 5040 ticks/q = 180000 mq/min |

**Consequence to keep in mind:** the same number means a different speed
when the beat changes. A 4/4 section at 120 beats/min followed by a 6/8
section at 120 beats/min goes from quarter-note beats to dotted-quarter
beats: 120 dotted quarters/min × 1.5 q/dotted quarter = 180 q/min, so
the quarter notes speed up from 120 q/min to 180 q/min. The song file
states each section's tempo in its own beats, so this is always explicit.

*Open:* whether decimal tempos (`112.5 beats/min`) are allowed; with a
sixteenth beat (250 mq/min per beat/min), a tempo with more than one
decimal place can fall between whole mq/min (112.25 beats/min × 250 =
28062.5).

## 14. Names in the code

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
- "Tick" means only the unit of 1/5040 of a quarter note. MIDI's clock
  messages, sent at 24 clock pulses/q, are always **clock pulses**;
  "pulse" alone always means a pulse of the bar (section 2).
- **Decided (Jim, 7 Nov 2026):** "count" is no longer a term: it is
  replaced by *pulse* and *beat* (section 2), as musicians and the
  rudiments use them. The counter shows beats (EFS-1.3).
- "Instant" means a clock reading in ns; "duration" means a difference
  of instants.

## 15. Open decisions and facts to verify

The tick size (section 4) and the clock (section 5, D-8) are decided.

**Open, sending MIDI clock (V1):**

- **O-2, O-3, O-13** (Timing Clock while stopped, when to send Start
  around a count-in, when the heartbeat runs) — closed by D-10: the
  count-in is a section of the score, Timing Clock runs from Play to
  Stop, and Start is a phrase cue.
- **O-11** How the screen gets the anchor in effect when a new anchor has
  been made for a position not yet reached (section 11).

**Following an external clock (future, section 12).** Decided by Jim,
1 Nov 2026, and written into EFS-2.9: O-1 (the slider is disabled),
O-5 (Song Position Pointer is honoured), O-6 (1 s, to be tested), O-7
(the screen's transport is disabled and the screen follows), O-8
(following comes before drawing), O-9 (the clock is not passed on) and
O-10 (the clock source changes only while stopped), and O-12 (where to
start is set on the play screen for both clock sources, EFS-2.1). None
is still open.

**Facts to verify before relying on them:**

- **Order at equal instants.** Events stamped with the same instant
  (a Start cue on one output port and a Timing Clock on the clock
  output) are delivered in the order sent.
- **FH-2 clock while stopped — settled 2 Nov 2026.** The FH-2's Clock
  outputs give no clock pulses while its transport is stopped; its tempo
  display still measures them, and it forwards them to the disting NT
  (section 9).
- **Beat pulse into a divider.** An FH-2 gate driven by the beat pulse,
  into the Quad Clock Divider, with a reset cue into its reset: steady
  through stops, and on the beat at the music's first bar.
- **Start on one port.** A Start cue on an output port connected only to
  the SQ-1 starts the SQ-1 and nothing else, through the MIDI router and
  through a direct `aconnect` connection.
