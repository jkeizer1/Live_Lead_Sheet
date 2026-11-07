# retrigger_gap_test — explainer

Revision 6 — 5 Nov 2026 (gaps 960 to 1600 µs, heard on the Disting NT over USB)

## What it proves

What a driven device needs between two notes of the same pitch for its gate
to retrigger (Song Notation, Q11). On 5 Nov 2026 the Disting NT, fed the
Bass from `song_play_test`, gave no new gate for a note that started at the
instant the previous one ended, and a gate for every note after a rest. This
test sends the simplest possible case through our ALSA output, with no song
and no player, so the cause is narrowed down:

- **Gap 0 µs** is the touching case: the note off and the next note on at
  the same instant, as the player sends today. If gates are missing here
  too, the plain pair is enough to fail, with nothing of the song or the
  player in the way.
- **Gaps of 960, 1100, 1250, 1400 and 1600 µs** (the defaults; revision 4
  used 250, 500, 640, 960 and 1500 µs, revision 3 and earlier 1, 2, 5 and
  10 ms) put the note off that long before the next note on. The first group in which every note has a gate is the gap
  this device needs. 960 µs is the time a cable needs for one 3-byte
  message, and 640 µs for the 2 bytes of a Note On that follows in running
  status, which is possible when the Note Off is sent as a Note On with
  velocity 0 (MIDI 1.0 Detailed Specification; CA-033: 320 µs per byte).
  The NT handles MIDI every 500 µs.
- **`--on-before-off`** repeats the 0 µs group with each note on queued
  before the note off it touches. If this sounds different from the normal
  0 µs group, the order in which the two messages arrive matters to the
  device.

The program sends the same note, channel and velocity every time, so only
the gap changes between groups.

## How to run

```
cargo run --release -p alsa_output --example retrigger_gap_test -- [options]
```

The options and their defaults are in the doc comment at the top of
`main.rs`; the program also prints the settings in use before it starts.
It shows its one port, `Bass`, and waits for Enter while you connect it
(the MIDI router suggests the id `lead-sheet-bass`) to the Disting NT as
in `song_play_test`.

What to listen for: six groups of eight short notes at 120 q/min (one
note every half a quarter note), a pause of eight notes' length between
groups, and a line on the terminal at the start of each group. Count the
notes in each group that have a gate. Then run

```
cargo run --release -p alsa_output --example retrigger_gap_test -- --gaps-us 0 --on-before-off
```

and count again.

Try a longer gap too (for example `--gaps-us 5000,20000`) if none of the
six works. The gap must be shorter than an eighth note (250000 µs at 120
q/min).

Whether a gap of a few hundred µs is really delivered that far apart
depends on the ALSA queue's timer; the program prints the warnings from
opening the sequencer, so read those first.

## Long run with a capture (the test used to choose the 5 ms floor)

Everything needed is here, so the test can be repeated on another device
without the session that first ran it. Use the MIDI router for the
connection (the Bass route to the device under test), not a direct ALSA
connection; the router must pass only the voice under test, so nothing
else sounds.

**1. The timing program.** 480 notes, one every 250 ms (120 q/min), 2
minutes, with a fixed gap (`--gaps-us` takes one value, in µs):

```
cargo run --release -p alsa_output --example retrigger_gap_test -- --gaps-us 5000 --notes 480 --channel 2 --note 24
```

Change `--channel` and `--note` to what the device listens to (`--channel`
counts from 1, as on the device). It prints the settings, shows its port
`Bass` in client `Lead Sheet`, and waits for Enter. Connect the port
(router or `aconnect`), then press Enter. Run it for gaps of 0, 5000 and
20000 on each device; add `--on-before-off` to the 0 run if you want that
order too.

**2. The capture** (optional but useful; second terminal, started before
Enter). First find the client number of `Lead Sheet` and use it as
`CLIENT:PORT` (the program prints it as well):

```
aconnect -l
```

```
stdbuf -oL aseqdump -p CLIENT:PORT | while IFS= read -r line; do printf '%s %s\n' "$(date +%T.%6N)" "$line"; done | tee capture.txt
```

Press Ctrl+C when the program ends. The shell adds the time stamp, which
is accurate to about 1 ms, so the capture shows the 250 ms grid and the
count of messages but cannot show a gap of a few hundred µs. Check the
capture:

```
grep -c "Note on" capture.txt
grep -c "Note off" capture.txt
awk '/Note on/ { split($1, t, ":"); s = t[1]*3600 + t[2]*60 + t[3]; if (n++) { d = (s - prev) * 1000; sum += d; if (min == "" || d < min) min = d; if (d > max) max = d } prev = s } END { printf "note ons %d, spacing ms: mean %.1f, min %.1f, max %.1f\n", n, sum/(n-1), min, max }' capture.txt
```

A clean run gives 480 Note Ons, 480 Note Offs, a mean of 250.0 ms and a
spacing within about 249 to 251 ms.

**3. What to listen for or record.** Count the notes that have a gate. For a
device whose audio you can record (the Bass on the NT was recorded on the
L-12, effects off), a 2-minute recording lets an onset detector count the
hits exactly; otherwise listen for a missing or stretched note (a note
that sounds like a dotted eighth is a missing gate). Write down: the
device, the path (USB, DIN, through the FH-2, a MIDI to CV converter), the
gap, and how many gates were missing.

**4. The song.** Then play `songs/Demo.song` about ten times
(`cargo run --release -p song_text --example song_play_test`) and listen
for a missed gate on each voice.

## Findings

**5 Nov 2026, Bass to the Disting NT through Jim's MIDI router, heard
only (the NT's gate was not measured), 120 q/min, note 24 on channel 2:**

- Default run, note off queued before the note on: **gap 0 ms** gave one
  bass note and then silence; **gaps 1, 2, 5 and 10 ms** each gave every
  note. Nothing between 0 ms and 1 ms was tried.
- `--gaps-ms 0 --on-before-off` (each note on queued before the note off it
  touches): one pulse.

So a plain touching pair, sent straight through our ALSA output with no
song and no player, fails on this device, and a gap of 1 ms is enough to
make it work. This matches what `song_play_test` showed with the Bass
(`songs/Demo.song`). The router's log lists the messages in the right
order at the router; it cannot show gaps of a few ms (it shows tenths of a
second). Not found: whether the NT, the router or the USB MIDI path is
what merges the pair; the result holds for this path.

**5 Nov 2026, the same default run with the Bass sent to the FH-2
instead of to the NT** (router route `lead-sheet-bass` to `fh-in`, channel
2, notes; the FH-2 forwards MIDI out its DIN pins, wired to the NT's DIN
input; heard only): **every group worked, including gap 0 ms.** The only
change from the run above was the last hop into the NT: over its own USB
port (router to NT) a gap of 0 ms failed; over the FH-2's serial link
(31.25 kbaud, 320 µs per byte, so at least 640 µs between the end of a Note
Off and the end of the next Note On) it worked.

**5 Nov 2026, revision 4 defaults (0, 250, 500, 640, 960 and 1500 µs), Bass
straight to the NT over USB, heard only, run many times:** results were
unreliable at every gap except, Jim thinks, 1500 µs. Even 960 µs dropped
several pulses in some runs (never all but one). Some runs worked at 250 µs
and most did not. 640 µs and 960 µs were somewhat more reliable than the
shorter gaps, and not reliable enough. So over USB the NT does not retrigger
dependably at the spacing a cable gives; the next defaults go from 960 to
1600 µs.

**5 Nov 2026, revision 5 defaults (0, 960, 1100, 1250, 1400 and 1600 µs),
Bass straight to the NT over USB, heard only, three runs of 8 notes per
gap:** 0 µs failed every time (one note, or two pulses); 960 µs failed in
two runs (2 notes, "a few pulses") and worked in one, with uneven timing;
**1100, 1250, 1400 and 1600 µs worked in every run.** About 20% of longer
32-note runs at 960 µs had a stretched note (like a dotted eighth). So over
USB the edge sits between 960 and 1100 µs, unlike the FH-2's serial link,
where 0 µs worked. The NT runs four algorithms and the USB audio interface,
so its timing is not constant.

**5 Nov 2026, long runs (480 notes, 120 q/min, one note every 250 ms, 2
minutes), Bass straight to the NT over USB, recorded on the L-12 and counted
with an onset detector; the NT ran only the Bass algorithm and the USB audio
interface, the router blocked every other voice:**

| Gap | Hits heard of 480 | Spacing of hits | Delivered by our program |
|---|---|---|---|
| 1500 µs | 463 (17 missing: one at 31.1 s in the recording, then 8 of 9 notes missing at 38.8 to 40.8 s, then every second eighth note missing to 45.3 s, clean for the other 77 s) | on the 250 ms grid | not captured |
| 20 ms | 480 | 249 to 250 ms | 480 on, 480 off; 250.000 ms mean |
| 5 ms, run A | 480 | 249 to 251 ms | 480 on, 480 off; 249.999 ms mean |
| 5 ms, run B | 480 | 249 to 251 ms | 480 on, 480 off; 249.999 ms mean |

The shape of a hit (level and decay out to 240 ms) is the same in all four
recordings to within 1%, and the recordings are far below full scale, so no
recording clips. Decision: a floor of 5 ms in the ALSA output, which the
studio's `retrigger_gap` can raise. One lossy run at 1500 µs against three
clean runs at 5 and 20 ms is enough to decide; it is not proof.

**Tempo impression, closed 5 Nov 2026.** Jim felt the direct ALSA
connection played a slightly slower tempo than the run through the router.
Two `aseqdump` captures (router closed, 32 notes, 960 µs) show the program
delivering Note Ons 250 ms apart (mean 249.9 ms), so 120 q/min was exact.
Phone stopwatch times (direct 7.94 to 8.45 s, router 8.19 to 8.35 s for an
expected 8.0 s plus reaction time) did not separate the paths. Not
reproduced and not pursued further.

What the specifications say that bears on this (read 5 Nov 2026, in
`docs/MIDI_1-0_Specs`): MIDI 1.0 requires no gap between a Note Off and
the next Note On and says nothing of retriggering a sounding note; a Note
Off before the next Note On is the order that asks for a new attack (Mono
mode: Note On, Note On, Note Off is legato). A cable gives 960 µs for one
3-byte message. USB-MIDI carries no timestamps and expects many notes at
once. The NT manual (1.18) says its MIDI handling runs every 24 frames at
48 kHz, i.e. every 0.5 ms. Not shown: whether the FH-2 does anything to
the pair besides serialising it; whether gaps below 1 ms work over USB.

## Revisions

- 5 Nov 2026, revision 1: first version.
- 5 Nov 2026, revision 2: findings of the first run.
- 5 Nov 2026, revision 3: the run through the FH-2's serial link.
- 5 Nov 2026, revision 4: gaps are given in µs (`--gaps-us`), with new
  defaults; the findings above were run with `--gaps-ms`, which no longer
  exists.
- 5 Nov 2026, revision 5: the runs of revision 4 were unreliable, so the
  defaults are now 0, 960, 1100, 1250, 1400 and 1600 µs.
- 5 Nov 2026, revision 6: findings of the revision 5 runs; the tempo
  impression is closed.
