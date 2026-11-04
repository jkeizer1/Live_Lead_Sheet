# control_track_test — explainer

Revision 4 — 4 Nov 2026

## What it proves

That a control track (D-10) can start and lead the studio's followers:

- the count-in is ordinary score bars — the beat pulse plays, nothing
  else starts;
- a transport cue on the music's first bar starts an external sequencer
  (the SQ-1) on that downbeat, and a Stop cue on the last barline stops it;
- Timing Clock goes out on one port only, `Clock`, from the first count-in
  bar to the end;
- optionally, Stop then Start at chosen points sends the SQ-1 back to
  step 1 (`--restart-every-sixteenths`).

## How to run it

```
cargo run --release -p alsa_output --example control_track_test -- [options]
```

The options and their defaults are in the doc comment at the top of
`main.rs`; the program also prints the settings in use before it starts.
It shows its three ports (`Clock`, `Beat pulse`, `To SQ-1`) and waits for
Enter while you connect them.

**Through Jim's MIDI router**, which passes clock and transport only from
its clock master, add `--transport-on-clock`, make `Clock` the clock
master, and route it to the SQ-1's `CTRL` port:

```toml
[clock]
master = "ls-clock-transport"      # the id you gave Lead Sheet / Clock

[[route]]
from = "ls-clock-transport"
to = "sq-1-ctrl"
channels = "all"                   # required, even for clock and transport
messages = ["clock", "transport"]
```

In the router, `log clock 30s` shows the clock, Start and Stop as they
pass.

**With direct connections** (`aconnect`), leave the option off and connect
`To SQ-1` and `Clock` both to the SQ-1's `CTRL` port.

**Restart test:** `--restart-every-sixteenths N` sends Stop then Start
every N sixteenths from the music's first bar. Configured as an 8-step
sequencer, the SQ-1 steps in eighths (2 sixteenths a step), so
N = 2 × the number of steps to loop: 10 loops steps 1 to 5, 12 loops
steps 1 to 6.

## Findings

### 4 Nov 2026 — SQ-1 through the router

Setup: Jim's MIDI router, `--transport-on-clock`, `Clock` routed to
`SQ-1 _ CTRL` with clock and transport; defaults otherwise (120 q/min,
4/4, 1 count-in bar, 8 bars). FH-2 and disting NT on and connected but
not routed; the SQ-1's CV and gate drive a modular voice heard through
the disting NT.

From the router's `log clock 30s` (15:17):

| Logged | Expected | Measured |
|---|---|---|
| clock arriving 15:17:31.123, then Start 15:17:33.123 | 1 bar: 4 q × 0.5 s/q = 2 s | 2.000 s |
| Stop "at beat 32.0", 15:17:49.123 | 8 bars: 32 q × 0.5 s/q = 16 s | 16.000 s, 32.0 beats |
| clock 120.0 BPM, 48 ticks a second, 20.0–21.0 ms between ticks | 24 pulses/q × 2 q/s = 48 pulses/s, 20.833 ms each | on the ALSA queue's 1 ms grid (D-8) |

- No Start during the count-in: the 2 s before it carried clock only.
- The SQ-1's own clock stopped arriving 0.5 s after ours began
  (15:17:31.623) and resumed about 0.2 s after the program closed
  (15:17:49.352): it followed our clock, then went back to its own.
- Heard by Jim: the SQ-1 responds as expected.

### 4 Nov 2026 — SQ-1 restarts

Same setup, with `--restart-every-sixteenths`. Heard by Jim:

- The SQ-1, configured as an 8-step sequencer, steps in eighths when
  following MIDI clock: 8 steps × 2520 ticks/step = 20160 ticks, one bar
  of 4/4. The SQ-1 can also run two sequences of up to 8 steps on
  separate channels, or one of up to 16; whether the step length can be
  set, or differs in those configurations, is not checked (manual not
  read).
- With 10 it went back to step 1 after step 5; with 12, after step 6:
  10 sixteenths ÷ 2 sixteenths/step = 5 steps, 12 ÷ 2 = 6 steps.
- So Stop then Start at the same instant, Stop first, resets the SQ-1 to
  step 1, with no gap needed between them.

### 4 Nov 2026 — very fast tempos

Same setup, `--restart-every-sixteenths 16 --music-bars 20`, the SQ-1 set
to 16th-note steps, its gate also triggering a VPME.QD drum voice. Heard by
Jim: the SQ-1, the drum voice and the beat pulse kept up, the beats exactly
where they should be, up to 22000 q/min or a little higher.

Above about 22000 q/min the run sometimes stopped:
`snd_seq_drain_output failed with error 'Try again (11)'`. Not a timing
fault: the kernel's output pool for our client was full.

- Events per second at 22000 q/min: Timing Clock
  22000 q/min × 24 pulses/q ÷ 60 s/min = 8800 pulses/s, beat pulse
  366.7 counts/s × 2 events = 733 events/s; about 9533 events/s in all
  (about 9750 at 22500 q/min).
- Events held at once: rate × (50 ms lead + up to 5 ms send loop + up to
  1 ms delivery) ≈ 9533 events/s × 0.050 s to 0.056 s = 477 to 534 events.
- That fits a pool of about 500 events (ALSA's default; to confirm with
  `cat /proc/asound/seq/clients` during a run): just under at steady state,
  just over at the send loop's worst moment, hence "sometimes".
- The test stopped because it treated "pool full" (EAGAIN) as a failure.

Fixed the same day: the output now asks the kernel for a 2000-event pool
(about 35000 events/s) and warns if refused, and a full pool is waited out
(`FlushOutcome::PoolFull`) instead of stopping the run; the test reports
how often the pool was full.

Re-run the same evening, `--restart-every-sixteenths 16 --music-bars 1400`,
with `cat /proc/asound/seq/clients` during each run:

| Tempo | Events/s (clock + beat pulse + restarts) | Predicted peak, × 0.051 s | Measured peak cells (pool 2000) | Failures |
|---|---|---|---|---|
| 20000 q/min | 8000 + 667 + 167 = 8833 | 450 | 451 | 0 |
| 30000 q/min | 12000 + 1000 + 250 = 13250 | 676 | 677 | 0 |

- The pool holds events/s × 51 ms (50 ms lead + 1 ms delivery grid); the
  send loop's 5 ms adds nothing. The 500-event default therefore runs out
  at about 500 ÷ 0.051 s ≈ 9800 events/s; 2000 events at about 39000
  events/s.
- The program reported no full pool and no late events at either tempo.
- The MIDI router's input pool (200 cells) peaked at 136, with no
  failures, at 13250 events/s.
- Heard by Jim at 30000 q/min: some disruption in the sound. Not
  pursued: the stress limits found are far above musical use, and finer
  boundaries are not worth the time.

This matters: a sequencer clocked far above the song's tempo becomes an
oscillator, its gate a waveform or a trigger for fast voices — a "sound
blast" from a cue. See D-10 ("Pulse bursts", later).

### 4 Nov 2026 — without `--transport-on-clock`

An earlier run without `--transport-on-clock` sent Start and Stop on
`To SQ-1`; the router passes transport only from its clock master
(the router's EFS-5.2), so the SQ-1 never started. Hence the option, and
D-10's two kinds of transport port.

## Not yet tested

- The beat pulse into the modular (FH-2 gate on channel 15 into a clock
  divider) and the reset trigger (channel 16).
- Hearing whether the SQ-1's first step lands on the downbeat (the beat
  pulse made audible is the reference).
- The direct `aconnect` path, without the router.
- The cause of the disruption heard at 30000 q/min (not to be pursued
  unless pulse bursts need it).
