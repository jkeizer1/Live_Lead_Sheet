# ALSA output — test results and interpretation

Revision 1 — 29 Oct 2026

The record of every run of the programs in this folder, with what each
result means. How to run them again, and what the terms mean, is in
[explainer.md](explainer.md). The decision these results support is D-8
(`docs/Architecture/Architecture.md`; reasoning in
`docs/Architecture/Musical Time and Clock.md`, section 5).

New runs are added at the end of section 2, with their date and what else
was running, so results stay comparable.

## 1. Conditions

| Item | Value |
|---|---|
| Date | 29 Oct 2026 |
| Machine | Jim's development machine: Linux Mint 22, kernel 7.0.0-42-generic, AMD Ryzen 9 3900X class CPU (12 cores, 24 hardware threads), run from a Cinnamon/X11 desktop |
| Build | `--release` for every measurement; debug for `click` |
| Studio | Powered off for the measurements; the FH-2 and disting NT were connected for the FH-2 test only |
| Drawing | None yet — the render half of the spike does not exist. "Load" below means busy threads (`--load N`), a stand-in |
| Kernel timer settings | Default timer for new queues: global HR timer (`snd_seq` parameters `seq_default_timer_class` 1, `seq_default_timer_device` 3); `seq_default_timer_resolution` 0; the kernel is built with `CONFIG_HZ` 1000 and `CONFIG_SND_SEQ_HRTIMER_DEFAULT` |
| Pulses | 3000 per row (about 62 s at 120 q/min, about 31 s at 240 q/min) |

## 2. Results

### 2.1 `click` — does the output work?

Studio off, no destination (the clicks go nowhere; lead still measured):

```text
click 0: stamped 50007 µs ahead of the clock
click 1: stamped 49772 µs ahead of the clock
click 2: stamped 49814 µs ahead of the clock
click 3: stamped 49814 µs ahead of the clock
click 4: stamped 49872 µs ahead of the clock
click 5: stamped 49933 µs ahead of the clock
click 6: stamped 49912 µs ahead of the clock
click 7: stamped 49921 µs ahead of the clock
```

No timer warning appeared. Connected to the FH-2 (`click 32:0`), its MIDI
monitor showed the note on and note off for note 76, velocity 100, on
channel 10, about eight times.

**Interpretation.**

- The adapter works end to end: sequencer, port, queue, real-time
  stamping, and delivery to a real device.
- The lead is 50 ms minus 67 to 228 µs. The shortfall is the sleeping
  thread waking late plus the time to send. It is harmless: the kernel
  delivers on the stamp, not on our wake-up.
- Click 0 was 7 µs *over* 50 ms. A sleep cannot wake early, so this is
  most likely the sleep's clock and the queue's clock differing by a few
  µs. It is noise, worth watching only if it grows.

### 2.2 `delivery_spread` — the queue (the adopted design)

| Run | Minimum | Median | p99 | p99.9 | Worst | Jitter (p99 − min) | Interval between pulses as received |
|---|---|---|---|---|---|---|---|
| 120 q/min, no load | 0.126 ms | 0.626 ms | 0.960 ms | 0.960 ms | 0.961 ms | 0.833 ms | 19.998 to 21.001 ms |
| 240 q/min, 16 busy threads | 0.015 ms | 0.515 ms | 0.932 ms | 0.932 ms | 0.932 ms | 0.916 ms | 9.999 to 11.000 ms |
| 240 q/min, 32 busy threads | 0.080 ms | 0.580 ms | 0.997 ms | 0.997 ms | 0.997 ms | 0.916 ms | 9.999 to 11.000 ms |

**Interpretation.**

- Delay is spread evenly from about 0 to 1 ms, and the pulse intervals are
  whole-millisecond steps around the ideal (20.833 ms at 120 q/min, 10.417
  ms at 240 q/min). The queue releases each message at the next 1 ms tick
  after its stamp. A pulse stamped just before a tick is delivered almost
  at once; one stamped just after waits almost 1 ms.
- Load changes nothing: the worst case was 0.961 ms idle and 0.997 ms with
  32 busy threads. The kernel delivers independently of our threads, which
  is the queue's advantage.
- The 0.5 ms median lateness is harmless: every message is late by about
  the same amount, so the tempo and the relationships are unaffected.
- Jitter is 0.8 to 0.9 ms: under the 1 ms pass mark used by the Linux MIDI
  Router's spike 1, but not by a large margin.
- **Not what we expected.** The start-up check reads `/proc/asound/seq/timer`,
  which said "HR timer" for our queue, and the HR timer counts in 1 ns
  steps. Yet delivery is on a 1 ms grid. Why the sequencer's queue ticks at
  1 ms with the HR timer is not explained (see section 3).

### 2.3 Explicit HR-timer selection (Python, from the Linux MIDI Router spike 1)

Run in `~/Documents/Linux_MIDI_Router/spikes/spike-1-clock-jitter`: the test
clock sends at 120 q/min, and `measure_latency.py` reports the pulse
intervals as received. Only the sender's timer option differs.

| Clock source | Pulse intervals as received (ideal 20.833 ms) |
|---|---|
| default ("1 ms system timer", as the program reports it) | 19.004 to 21.996 ms |
| `--high-resolution-timer` (queue timer set explicitly) | 19.999 to 21.024 ms |
| our Rust queue, default (section 2.2) | 19.998 to 21.001 ms |

**Interpretation.** Selecting the HR timer explicitly improves on that
program's default (which is the 1 ms system timer, about ±2 ms) and lands
exactly where our Rust queue already is. So explicit selection would
change nothing for us on this machine, and the option of adding `unsafe`
code and a dependency to do it was dropped. The Python README's remark
that the high-resolution baseline "should be well within ±1 ms of ideal"
does hold, but only just: it is the same 1 ms grid.

### 2.4 `deadline_spread` — the fallback (not adopted)

The program sleeps until each pulse's instant, reading the queue's clock,
and sends the pulse at once.

| Run | Minimum | Median | p99 | p99.9 | Worst | Jitter (p99 − min) | Interval between pulses as received |
|---|---|---|---|---|---|---|---|
| 120 q/min, no load, sleep | 0.027 ms | 0.058 ms | 0.062 ms | 0.069 ms | 0.871 ms | 0.035 ms | 20.019 to 21.646 ms |
| 120 q/min, no load, sleep + 200 µs busy-wait | 0.000 ms | 0.001 ms | 0.003 ms | 0.509 ms | 1.805 ms | 0.002 ms | 19.029 to 22.637 ms |
| 240 q/min, 32 busy threads, sleep | 0.055 ms | 0.057 ms | 0.062 ms | 2.970 ms | 2.973 ms | 0.006 ms | 7.500 to 13.330 ms |
| 240 q/min, 32 busy threads, sleep + 200 µs busy-wait | 0.000 ms | 0.001 ms | 2.652 ms | 4.069 ms | 6.321 ms | 2.651 ms | 4.096 to 16.736 ms |

**Interpretation.**

- For 99% of pulses, sleeping to the deadline is about ten times more exact
  than the queue: the typical delay is 58 µs against the queue's 0.5 ms,
  and 99% of pulses are within 62 µs of the instant.
- The tail is the problem. With 32 busy threads, about 1 pulse in 1000
  arrived up to 3 ms late, and even with no load one pulse in 3000 was
  0.87 ms late. The queue never exceeds 1 ms.
- Busy-waiting the last 200 µs is excellent on a free CPU (median 1 µs) and
  bad on a busy one: the waiting thread is descheduled by the operating
  system, giving p99 2.65 ms and a worst case of 6.3 ms, worse than the
  queue at every percentile above the median.
- The ideal-interval comparison tells the same story: the received
  intervals range 7.5 to 13.3 ms around 10.417 ms (sleep) and 4.1 to 16.7
  ms (spin) under load, against the queue's 9.999 to 11.000 ms.

## 3. Overall interpretation

**Decision supported: the queue stays the one clock (D-8).** Its
behaviour is coarse (a 1 ms grid) but bounded and independent of what the
program is doing. The alternative is far more exact most of the time and
much worse in rare moments, and a rare late pulse is the more audible
fault than a constant small wobble.

**Is the worst case audible?** The queue's worst case, 1 ms, is below what
listeners are generally reported to notice. From memory of the
timing-perception literature (not checked for this record): a trained
listener detects a timing deviation in a steady rhythm from about 2 to
5 ms, and a hit early or late by under 10 ms is heard as feel, not as
error. USB MIDI delivers in 1 ms frames and a MIDI cable takes 0.32 ms per
byte, so the followers' own timing is already about as coarse. The worst
pulse of any run, 6.3 ms, is at the edge of what a trained listener might
notice, and only if the follower steps on that very pulse. The listening
test has not been done; this is a judgement from numbers, not a hearing.

**Limits of this evidence.**

- The load is artificial. 32 busy threads on 24 hardware threads is harsher
  than the real program; real drawing is one thread mostly waiting on the
  GPU.
- 3000 pulses give only 3 samples at p99.9. Tail figures are rough; the
  explainer gives the `--pulses 30000` run for better ones.
- The machine was otherwise idle. A game running, a browser, or PipeWire
  at a small buffer size could change the fallback's tail far more than
  the queue's.
- The receiving client is part of the same program. It sleeps 1 ms between
  checks when idle; that does not affect the kernel's arrival stamps, but
  it is one more thread competing for the CPU.

**Open questions.**

1. Why does the sequencer queue tick at 1 ms with the HR timer selected
   (default or explicit)? The `snd_seq` module parameters show a default
   resolution of 0 and nothing else that explains it. It does not block
   V1, because the 1 ms bound is acceptable, but a different kernel or
   setting might give finer delivery for free. Worth a look in the kernel
   sound sources if finer timing is ever wanted.
2. What does it sound like? The listening test through the FH-2 and the
   followers Jim uses, with real drawing load, is outstanding.
3. Does real drawing load move the queue's figures? Re-run the
   `delivery_spread` rows while the render half of the spike is drawing
   (explainer section 5).
