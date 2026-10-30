# The ALSA output examples — what they do and how to run them

Revision 1 — 29 Oct 2026

Three small programs that try the MIDI output on the real machine. They
exist to answer one question for decision D-8: *can the ALSA sequencer's
own clock keep time well enough for a live lead sheet?* The answer so far
is in section 5 of `docs/Architecture/Musical Time and Clock.md`. This
page explains the ideas for someone who has not worked with ALSA, and
says exactly how to run the measurements again.

## 1. The ideas

**ALSA** is Linux's sound layer. Its **sequencer** is a MIDI switchboard
inside the kernel. Programs ("clients") connect to it, each with one or
more "ports". Connecting one client's port to another's makes MIDI flow
from the first to the second. `aconnect -l` lists the clients and ports;
`FH-2` and `disting NT` are clients created by the kernel for your USB
devices. Think of it as a message bus: our program publishes MIDI
messages, the bus delivers them to whoever subscribed.

**A queue** is a timed mailbox inside the sequencer. Instead of sending a
message "now", a client puts it in the queue with a time stamp, and the
kernel delivers it when the queue's clock reaches that time. This is the
point of using it: once a message is in the queue, the kernel delivers
it, even if our program is busy drawing the screen at that moment.

**The clock** (D-8). The queue counts real time in nanoseconds (ns).
Our program uses *that* count as its one clock: it asks the queue "what
time is it?", works out the time for each musical event, and stamps the
event with it. A reading of this clock is an **instant** (type
`Instant`); a span between two instants is a **duration** (type
`Duration`). The queue has no tempo of its own; tempo only exists in our
own arithmetic (see `Musical Time and Clock.md`, section 6).

**The lead window.** The program hands each message to the queue about
50 ms before it is due. Too early and a tempo change cannot take effect
on messages already handed over; too late and a busy moment could make
us miss the time. Fifty milliseconds is the agreed compromise.

**MIDI clock pulses.** Gear that follows a master (a sequencer, an
arpeggiator) is told the tempo by a stream of 24 identical one-byte
messages per quarter note. At 120 q/min a pulse is due every 20.833 ms.
Pulses are the most timing-sensitive thing we send, so the measurements
use them. (Here a *pulse* is a MIDI clock message, never a *tick*; a tick
is the score unit of 1/5040 of a quarter note.)

**What "timing quality" means here.** The queue's timer runs in steps.
The question is how late after its stamped instant a message actually
leaves. **Delay** = (instant the message actually arrived) − (instant it
was stamped for). The **jitter** is how much the delay varies:
p99 − minimum, that is, the spread that covers 99 of every 100 pulses.
A constant delay is harmless (everything is late together); a varying
delay is what makes the beat wobble.

**How the measurement stays honest.** Each example starts a second ALSA
client of its own that receives the pulses. The kernel stamps each
arrival with the queue's clock at the moment of arrival, so our own
program's scheduling (which is what we are testing) cannot distort the
reading. The method is borrowed from the Linux MIDI Router's spike 1.

## 2. The three programs

All are in this folder. `measurement/mod.rs` holds the code the two
measuring programs share (the receiving client, the load, the report),
so their numbers are directly comparable.

| Program | What it does |
|---|---|
| `click.rs` | Plays eight clicks, one every 500 ms, each stamped about 50 ms ahead, and prints how far ahead of the clock each was stamped. Proves the output reaches a real device. |
| `delivery_spread.rs` | Sends 3000 clock pulses through the queue (the adopted design) and reports the delay of each. |
| `deadline_spread.rs` | The fallback: the program itself sleeps until each pulse's instant and sends it at once. Reports the same figures. Not adopted, kept for comparison. |

## 3. Running them

All commands are run from the project folder
`~/Documents/Live_Lead_Sheet`. Measuring programs always run with
`--release` (optimised code; debug builds would distort timing).
Everything after the lone `--` goes to the program, not to `cargo`.

**Before measuring:** close anything heavy you do not want in the
result (browser tabs, a game), unless the run is meant to include it.
No MIDI device needs to be connected for the measurements; for `click`,
connect a device.

### 3.1 `click`

```bash
aconnect -l                                   # find the device, e.g. FH-2 is 32:0
cargo run -p alsa_output --example click 32:0
```

Expect eight lines `click N: stamped NNNNN µs ahead of the clock`, with
values just under 50000. With no destination argument the clicks go
nowhere but the lead is still printed. The clicks are note 76 on MIDI
channel 10; your device must listen to that channel to make a sound.

### 3.2 `delivery_spread` (the queue)

```bash
cargo run --release -p alsa_output --example delivery_spread -- [--bpm N] [--pulses N] [--load N]
```

| Option | Meaning | Default |
|---|---|---|
| `--bpm N` | quarter notes per minute, whole number | 120 |
| `--pulses N` | how many pulses to measure (at least 10) | 3000 |
| `--load N` | N busy threads competing for the CPU, standing in for drawing | 0 |

### 3.3 `deadline_spread` (the fallback)

```bash
cargo run --release -p alsa_output --example deadline_spread -- [--bpm N] [--pulses N] [--load N] [--spin-us N]
```

As above, plus `--spin-us N`: spend the last N µs before each pulse
busy-waiting instead of sleeping (more exact on a free CPU, worse on a
busy one). Default 0.

### 3.4 The full matrix

Paste these one after another. The first three are the queue, the rest
the fallback; the same rows were run on 29 Oct 2026.

```bash
cargo run --release -p alsa_output --example delivery_spread
cargo run --release -p alsa_output --example delivery_spread -- --bpm 240 --load 16
cargo run --release -p alsa_output --example delivery_spread -- --bpm 240 --load 32
cargo run --release -p alsa_output --example deadline_spread
cargo run --release -p alsa_output --example deadline_spread -- --spin-us 200
cargo run --release -p alsa_output --example deadline_spread -- --bpm 240 --load 32
cargo run --release -p alsa_output --example deadline_spread -- --bpm 240 --load 32 --spin-us 200
```

A run of 3000 pulses takes about 62 s at 120 q/min and 31 s at 240 q/min.
For the tail (the rare late pulse), use `--pulses 30000`: about ten
minutes at 120 q/min. 32 busy threads on the Ryzen 3900X's 24 hardware
threads is harsher than the real program will be; use `--load 16` for a
gentler test.

## 4. Reading the output

```text
3000 pulses at 240 q/min, 32 load threads, final 0 µs busy-waited
Delay of delivery after the scheduled instant
  minimum            0.080 ms
  median (p50)       0.580 ms      half the pulses were later than this
  p99                0.997 ms      99 of every 100 pulses were on time or earlier
  p99.9              0.997 ms
  maximum            0.997 ms      the single worst pulse
  jitter (p99 - min) 0.916 ms      the number to compare between runs
Interval between pulses as received: 9.999 ms to 11.000 ms
```

- The last line compares with the ideal interval between pulses: 20.833 ms
  at 120 q/min, 10.417 ms at 240 q/min. Steps of exactly 1 ms around
  the ideal mean the sender is working on a 1 ms grid.
- A `WARNING:` line at the top means the queue is not on the HR timer, so
  the run is not valid; send the text to Jim.
- An error such as "pulse N arrived with tag …" means a pulse was lost or
  reordered; the run stops instead of reporting wrong numbers.

**The pass mark** is the one the Linux MIDI Router spike used: jitter under
1 ms in every row. The worst case also matters more than the typical one:
a rare late pulse is what a listener could hear.

### Results so far (29 Oct 2026, idle desktop, no real drawing)

| Sender | Tempo, load | Jitter (p99 − min) | Worst |
|---|---|---|---|
| Queue (adopted) | 120 q/min, none | 0.833 ms | 0.961 ms |
| Queue | 240 q/min, 32 threads | 0.916 ms | 0.997 ms |
| Sleep to deadline | 240 q/min, 32 threads | 0.006 ms | 2.973 ms |
| Sleep + 200 µs busy-wait | 240 q/min, 32 threads | 2.651 ms | 6.321 ms |

The queue is coarse (a 1 ms grid) but its worst case never exceeds 1 ms,
whatever the load, so it was kept. Full table and reasoning:
`Musical Time and Clock.md`, section 5.

## 5. When to run them again

The numbers above come from artificial load. Run the matrix again, and
compare with the table above, at each of these points:

1. **When the render half of the spike exists** (the 8-bar strip at
   2560 × 1440 drawing at full frame rate). Start the drawing program in
   one terminal and, while it runs, `delivery_spread` in another
   (`--bpm 120` and `--bpm 240`, no `--load`). The measuring program has
   its own queue, so this tests how heavily the drawing loads the
   machine, which is the thing that can disturb timing. A jitter near
   1 ms with a worst case near 1 ms means the design holds.
2. **When the real player sends MIDI clock.** Add a measuring mode to the
   player itself, so the program's real sending path is measured and not
   only this stand-in. (Not written yet.)
3. **After any change to the kernel, the audio set-up (PipeWire, JACK)
   or the CPU governor,** and on any other machine the program must run
   on.
4. **Listening test.** The numbers say how even the timing is, not
   whether it sounds right. Send clock to the follower you actually use
   (the FH-2 into a modular sequencer, an arpeggiator on external clock)
   while the program draws, and listen at 120 and 240 q/min. This has not
   been done for this program yet; it is the final check.

Record each new run in `Musical Time and Clock.md` section 5 (new rows in
the "Delivery measured" table), with the date and what else was running.

## 6. If something goes wrong

| Message | Meaning and fix |
|---|---|
| `open /dev/snd/seq failed: No such file or directory` | The ALSA sequencer is not available (for example inside a restricted shell). Run in a normal terminal. |
| `MIDI timing may be uneven: queue N runs on 'system timer'` | Load the high-resolution timer: `sudo modprobe snd-hrtimer`, then run again. |
| `Receiver: no pulse for 2 s` | Pulses did not reach the receiving client; the run stops. Run again; if it repeats, tell Jim. |
| `connecting our port … failed` | The destination in `click` does not exist. Check `aconnect -l`. |
