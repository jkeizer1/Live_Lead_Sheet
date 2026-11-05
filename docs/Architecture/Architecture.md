# Live Lead Sheet — Architecture

Revision 20 — 5 Nov 2026 (the beat pulse is a mono voice, not a control-track feature)

Builds on the Concept Brief (Revision 13), the Experience Functional
Specification (Revision 13) and Song Notation (Revision 9). Sets the
principles, the main parts, how they talk, and the technology for V1.

**Focus.** V1 is the Live Lead Sheet itself: the leader, on one native
screen, on Linux. Everything below about chains, other platforms and
networks is there so V1 does not block it — not to be built now.

## Principles

**A-1 The vocabulary is the core.** At the centre is a symbolic musical
language: key, meter, tempo, chord, voicing, rhythm pattern, voice
track, control track, phrase cue, section, song, position. Everything
else is built around it. It represents music as a musician thinks of
it, not as any device or file encodes it.

**A-2 Meaning, abstracted; types, concrete.** The vocabulary is an
abstraction of *music* — that is where abstraction earns its keep. Each
concept in it is a distinct, purpose-named type with explicit fields: a
`Chord` is a root, a kind, a bass and its degrees, never a general-purpose
bag of values. Chord naming follows MusicXML's harmony model (root, kind,
bass, degree), so the meaning is already well defined and export is a
direct map.

**A-3 Encodings live at the edges.** The vocabulary has no wire or file
format of its own. Every encoding is an adapter that translates to or
from it: the song text file, MusicXML export, MIDI out, and later OSC,
WebSocket or MIDI 2.0 Flex Data. Committing the core to any one encoding
would narrow it prematurely. *(In EAI terms: a canonical data model with
adapters at each endpoint.)*

**A-4 Transforms are maps on the vocabulary.** A transform takes the
vocabulary in and gives the vocabulary out: transposition that keeps to
the key, a voicing change, later a melody generator. Transforms chain.
Rendering to MIDI is simply the last map in a chain — vocabulary in,
MIDI out.

A transform maps **both** the song and the events (A-6). Its output is a
transformed song plus transformed events, so every follower after it
still knows what is coming — including a key change the transform will
make. A transform whose future is not decided in advance (*e.g.* a
generator) publishes as far ahead as it has decided, and no further.

**A-5 Publish, then follow.** The song player publishes; everything else
follows. Transforms, the lead sheet view and the MIDI output are all
followers. The test of the design: the lead sheet view is drawn *only*
from what is published. If it needs something that isn't published, the
vocabulary is incomplete.

**A-6 Two kinds of information.** The **song** — the whole of it, as a
document, given to a follower when it joins or when the song changes.
The **events** — started, stopped, section, repeat, bar, count, tempo,
phrase cue — as they happen. A follower knows what is coming by
combining the two; the future is never streamed.

**A-7 Musical time first.** Inside, time is musical: bar, count, and
exact fractions of a note. Triplets and dotted values are exact, with no
rounding drift. Clock time (milliseconds) appears only where music meets
hardware or screen: the MIDI sender and the display. One clock, read by
the player and supplied by the MIDI output adapter, sets the tempo for
everything.

**A-8 Keep playing.** Senders and receivers are independent. A missing
receiver means messages go nowhere, never that playing stops. Every
failure is reported in the warnings region; none is silent, none is
audible.

**A-9 Nothing generated in the core.** The core plays exactly what is
written. Generation, when it comes, is a transform outside the core.

**A-10 Transforms can be scoped in time.** A transform need not apply to
the whole song. It can apply to chosen sections, chosen repeats, or a
range of bars, and the music reverts outside that scope — *e.g.*
transpose section B up a whole step on its third repeat only, to break
up repetition. Scope is expressed in the vocabulary's own terms
(section, repeat, bar), so it is known ahead and shown ahead.

**A-11 Room to grow.** V1 builds only the leader, but nothing in it may
block what is planned:

| Later | Fits as |
|---|---|
| Listener | another publisher of the same vocabulary, fed by MIDI in |
| Recorder | a follower that writes what it hears into a song in the catalog |
| Melody generator, other transforms | a link in the chain, with its own view (A-4) |
| Strudel, other programs, remote players | links reached through a transport output (A-3) |
| Navigation marks (D.C., Coda…) | song structure; the player resolves play order |
| SATB, one channel per voice | an option of the MIDI output |
| Tunings | an attribute of the song, passed to instruments |
| More windows or monitors | more followers drawing parts of the same stream |

## Capabilities

**Song authoring.** Builds and edits songs from the catalogs, and stores
them in the song catalog.

**Song playing.** Walks a song in musical time, owns the clock and the
tempo slider, and publishes the song and its events.

**Transform chaining.** Passes what the player publishes through a chain
of links. A *transform* takes the vocabulary in and gives it out (A-4);
an *output* takes the vocabulary in and gives something else out — MIDI
to the studio, later a network transport. The chain is set up by the
musician; it may be as short as a single MIDI output.

## Windows

Each capability, and each transform, has its **own window**. Windows sit
side by side, and the musician can maximize any one to focus on it.

- The **song player's window** is the lead sheet view of the song *as
  written* — the start of the chain.
- A **transform's window** shows its input and its output as lead sheet
  views, together with its own controls and scope — *e.g.* a harmonic
  transposition window showing the song in G going in and in A coming
  out, for section B, repeat 3.
- Other windows — authoring, warnings — sit alongside.

*Example:* a melody generator's window and another generative window on
the left, the song player's window on the right.

## Catalogs

- **Song catalog** — the compositions.
- **Chord progression catalog** — progressions in Roman numerals, in
  groups.
- **Rhythm pattern catalog** — one- to eight-bar patterns, in groups.

All three hold the vocabulary, stored as text files through a file
adapter (A-3). Beside them stands the **studio configuration**: not
music, so not in the vocabulary, but the setup that realises a song in one
studio — ports, channels, octave offsets, velocity levels and device
timing (D-11). It is a text file read through the same adapter.

## Information flows

Authoring and playing:

```
Chord progression catalog ─┐
Rhythm pattern catalog ────┼─▶ Song authoring ─▶ Song catalog ─▶ Song player
Song catalog (to edit) ────┘     (view)                          (owns the clock)
```

The chain — vocabulary flows left to right until an output leaves it:

```
Song player ─▶ Transform ─▶ Transform ─▶ … ─▶ MIDI output ─▶ ALSA ─▶ MIDI router ─▶ devices
               (e.g. melody   (e.g. other        notes, clock,          (or direct
                generator,     generative,       beat pulse,             connections,
                own view)      own view)         phrase cues             D-10)
```

Each link's window shows what goes into it and, for a transform, what
comes out.

Documents leave through their own adapter:

```
Song catalog ─▶ MusicXML export ─▶ file for notation software
```

Every capability reports failures to the **warnings** region (A-8).

## V1

V1 builds song authoring, song playing, the three catalogs, a chain
holding only the MIDI output, the song player's window, warnings, and MusicXML
export. The chain itself exists in V1 — empty of transforms — so adding
the first transform later adds a link rather than reworking the parts.
Inside V1 the links are within one program; reaching other programs
later means adding a transport output.

## Technology — V1

**Language: Rust.** Its enums with exhaustive matching suit the
vocabulary (add a chord kind and the compiler lists every place that must
handle it), and it catches data races between the timing and drawing
threads at compile time. Strict: compiler warnings as errors,
`clippy::pedantic`.

**Dependencies: a written allow-list.** Each crate is justified before it
is added; `cargo tree` is checked on every change; `Cargo.lock` is
committed. Starting list:

| Crate | Purpose |
|---|---|
| `alsa` | MIDI out through the ALSA sequencer |
| `winit` (default features off; `x11`) | window and input |
| `wgpu` (pinned) | GPU drawing |
| `ttf-parser` | reading glyph outlines in the offline atlas tool only; not in the program (approved by Jim, 30 Oct 2026; harfbuzz organisation, no dependencies of its own) |

Crates are chosen for quality first and count second: standard, widely
used, maintained by an organisation or several people (CLAUDE.md; D-9).
Measured: 106 crates in all (103 without build tools), almost all of them
wgpu's and winit's. wgpu changes its major version every few months; it
is pinned and upgraded only by choice.

**GPU: WebGPU through wgpu.** A modern API at roughly OpenGL's level —
vertex and fragment shaders in WGSL — over Vulkan on Linux (and Metal or
Direct3D 12 later), without Vulkan's boilerplate. All GPU code lives in
the render crate.

**Rendering.** The CPU sends small arrays of instance records each frame;
three shaders draw everything:

- *glyph shader* — noteheads, clefs, accidentals, rests and chord-symbol
  text, from a signed-distance-field atlas (crisp at any size);
- *line shader* — staff lines, barlines, stems, beams, playhead;
- *wheel shader* — the key wheel's sectors, computed per pixel.

The atlas is built once, offline, from the music font (Bravura) and a
text font, and committed as an image plus a glyph table. The program
itself has no font library.

**Windows: native.** The song player's window, authoring and warnings are
native windows in V1. Authoring controls are drawn by the program, in
the same dark, soft-contrast style.

**Crates, by purpose.**

| Crate | Holds |
|---|---|
| vocabulary | the musical language (A-1, A-2), including musical time (D-7) |
| song_text | the parser and writer for song, studio configuration and library files (A-3, D-11); no external crate |
| player | reading the clock, publishing (A-5 – A-7) |
| render | layout (vocabulary → instance records) and GPU drawing |
| alsa output | the MIDI output (A-3), and the clock it supplies (A-7, D-8) |
| app | windows, authoring, wiring the parts together |
| atlas tool | offline: builds the glyph atlas from the fonts in `assets/fonts` (run by hand; not part of the program) |

The render crate takes a drawing surface and the published stream —
nothing else: no windowing, files or threads inside it. That keeps
layout testable without a GPU, and keeps later options (a browser client,
more windows) as wiring rather than rewriting.

**Platform code only in adapters.** The ALSA MIDI output is the only
Linux-specific part. Another platform later means another MIDI output
adapter.

## Constraints

- Linux Mint; MIDI through the ALSA sequencer, as with the MIDI router.
- Even tempo with no drift (EFS-2.4); timing is not disturbed by drawing.
- The display scales with screen size and resolution (EFS-1.12) and is
  drawn on the GPU, so a busy screen never costs timing.
- Dark theme with soft contrast, controls included.
- Strict language modes; every error handled and reported in full; no
  silent defaults.
- Each dependency justified against writing the small piece ourselves.

## Decision record

Why each choice was made, and what was set aside — so the reasoning
survives without the conversations behind it.

**D-1 Our own vocabulary, not MusicXML or MIDI, between links.**
MIDI carries notes but no chord names, key, sections, or knowledge of
what is coming. MusicXML has an excellent chord model but is a document
format: no notion of "now", no streaming, and no place for our concepts
(rhythm patterns assigned to bars, lead track, accent and articulation marks, phrase cues, the tempo slider). MIDI 2.0's Flex Data messages (chord name,
key, time signature, tempo) are real-time but cover only "now", with no
structure or look-ahead, and few receivers support them — kept as a
possible later output. Hence: our own vocabulary, MusicXML's harmony
names (root, kind, bass, degree) borrowed, encodings only at the edges.

**D-2 Rust.** Compared with C17 and Python:

- *C17* — fewest dependencies (libasound, GLFW, libGL from distro
  packages; a generated GL loader and `stb_truetype` vendored) and the
  most stable over years, but the compiler cannot force every musical
  case to be handled, and thread races are found only at run time.
- *Python* — timing thread and drawing in one process contend on the
  interpreter lock; set aside for this program (unlike the router).
- *Rust* — exhaustive matching for the vocabulary, data races caught at
  compile time. Its risk is dependency sprawl, managed by the allow-list.

**D-3 Measured crate counts (Linux, transitive, Oct 2026).**

| Stack | Crates |
|---|---|
| `glfw` (defaults off) + `gl` + `alsa` — OpenGL 3.3 | 19 |
| `glfw` (defaults off) + `wgpu` + `alsa` — first choice, replaced by D-9 | 73 (69 with only Vulkan and WGSL) |
| `winit` + `glutin` + `gl` + `alsa` | 85 |
| `glfw` (defaults on) + `gl` + `alsa` | 110 |
| `winit` (`x11`) + `wgpu` (Vulkan, WGSL) — chosen (D-9), without `alsa` | 106 (103 without build tools) |

`wgpu` alone is 58. Its major version was 0.19 in January 2024 and 30 in
October 2026: expect breaking releases every few months, hence pinning.

**D-4 wgpu (WebGPU) for the GPU.** Alternatives set aside:

- *OpenGL 3.3* — fewest crates and not deprecated on Linux or Windows,
  but Apple deprecated it in 2018 (frozen at 4.1); chose the modern API.
- *Vulkan directly (`ash`)* — few crates, stable, modern, but hundreds of
  lines of boilerplate (memory, synchronisation, image layouts) before
  the first triangle.
- *SDL3's GPU API* — one C library, WebGPU-like, but shaders must be
  supplied per backend (SPIR-V, Metal, DXIL) and its Rust binding's
  maturity is uncertain.

wgpu keeps the shader concepts of OpenGL (vertex, fragment, WGSL), hides
Vulkan's boilerplate, runs on Vulkan, Metal and Direct3D 12, and is the
same API browsers use.

**D-5 Shaders, not CPU drawing.** Drawing on the CPU and uploading the
frame (as the `pixels` crate does — it still needs wgpu underneath) costs
14.7 MB per frame at 2560 × 1440 (33 MB at 4K): about 1–2 ms to clear or
copy, 1–3 ms to upload, and far more to draw antialiased glyphs on one
core. With instanced shaders, a full redraw of a few thousand records
costs the GPU almost nothing, so no dirty-rectangle or layer caching is
needed. A CPU drawing library (`tiny-skia`) with a CPU presenter
(`softbuffer`) was considered and set aside for this reason.

**D-6 Native windows for V1, browser later.** A browser client is
possible: the render crate compiles to WebAssembly and the WGSL shaders
are the browser's own shading language. But as of October 2026, Chrome on
Linux enables WebGPU by default only on Intel Gen12+ and on NVIDIA under
Wayland — an AMD RX 9070 needs the `enable-unsafe-webgpu` flag — and
Firefox on Linux has it only in Nightly (wgpu could fall back to WebGL2).
A browser would make authoring controls easy, at the cost of an HTML and
JavaScript layer and a local web server. V1 stays native; the render
crate is kept browser-ready (no windowing, files or threads inside it).

**D-7 Musical time: whole ticks, 5040 ticks/q, in a `u64`.** Score
quantities (A-7) are whole numbers of ticks, so they add and compare
exactly and never drift. 5040 = 2⁴ × 3² × 5 × 7 divides a quarter note
exactly for every subdivision the music uses, and one MIDI clock pulse
is 210 ticks. Two types: `ScorePosition` (a place in the song, from
0 ticks at the first downbeat) and `ScoreLength` (a distance along it),
with only checked operations between them. Instants are computed from an
anchor, never by adding intervals, so rounding cannot accumulate
(EFS-2.4). Set aside: 960 ticks/q (no sevenths), fractions, and
floating-point seconds.

The full reference — definitions, conversions, worked examples and the
reasons for each choice — is
[Musical Time and Clock](Musical%20Time%20and%20Clock.md).

**D-8 Keeping time: the ALSA queue's real time is the one clock.**
Decided by Jim, 29 Oct 2026. The player converts every position to an
instant from an anchor (D-7) and hands the event to the ALSA sequencer
queue up to 50 ms ahead, stamped in real time; the queue has no tempo,
and the kernel delivers each event between 0 and 1 ms after its instant
whatever the program is doing, so drawing cannot disturb timing. The same
queue's real time is read by the player for every conversion and by the
screen once per frame, supplied by the ALSA output adapter. The queue is
expected to use the HR timer (the default on this machine); the adapter
checks this at start-up by reading `/proc/asound/seq/timer` and reports
anything else in the warnings region. Explicit timer selection is not
needed: it was tried and changes nothing here (and the `alsa` crate
cannot do it without `unsafe` code and a new dependency).
Tempo changes and loop release take effect at the end of what is
already queued; Stop clears the queue. Set aside: a queue with its own
tempo (a second clock, which drifts from ours).

*Measured, 29 Oct 2026:* the queue's delivery is bounded at 1 ms and
unaffected by load (worst 0.997 ms, 240 q/min, 32 busy threads). The
fallback, a thread sleeping to absolute deadlines, was tested and not
adopted: ten times more exact for 99% of pulses, but up to 3 ms late once
in a thousand under heavy load, and 6.3 ms with busy-waiting. A 1 ms
bound is below what listeners generally notice; the final check is the
listening test with real drawing load.

*Other platforms (future work, not V1):* the pattern carries over
unchanged. Each operating system's own MIDI scheduler takes events stamped
with an instant and delivers them itself: on macOS, CoreMIDI (`MIDISend`
with a future `MIDITimeStamp`, in host time); on Windows 11, Windows MIDI
Services (scheduled, timestamped sending). Each platform gets its own
output adapter offering the same four things: read the current instant,
schedule an event at an instant, remove pending events, send at once.
Events are stamped in real time, not in ticks on a tempo queue: ALSA's
tick queue has no counterpart on the other platforms. Only an output with
no scheduler of its own (legacy Windows `midiOut`, which sends at once)
needs a sender thread sleeping to deadlines, written in that adapter
against that adapter's clock. Such a thread is bounded only at real-time
thread priority (the measurements above are at normal priority), so that
adapter would be measured first. A sleep crate such as `spin_sleep` was
considered and set aside: it is the sleep-then-busy-wait method measured
above, it waits on a second clock (`std::time::Instant`), and it has a
single author.

The full decision — reading the clock, the timer evidence, re-anchoring,
loop release and stop, sending MIDI clock — is in sections 5, 8 and 9 of
[Musical Time and Clock](Musical%20Time%20and%20Clock.md).

**D-9 Window: `winit`, not `glfw`; crate quality before crate count.**
Decided by Jim, 29 Oct 2026. The crate rule is: standard, widely used
crates maintained by an organisation or several people; avoid
single-author crates and abandonment risk, even at the cost of a higher
count (CLAUDE.md). Against it:

- `winit` (the `rust-windowing` organisation) is the standard window
  crate beside `wgpu` (Bevy, egui and wgpu's own examples use it).
- `glfw` is a thin Rust wrapper kept on a personal repository, over the
  well-supported C library GLFW. Besides the maintenance risk it did not
  build here: the crate wants GLFW 3.4, Ubuntu 24.04 ships 3.3.10, so it
  builds its bundled copy with `cmake`, which stops for missing X11
  development headers. `winit` builds with no system packages.
- Cost: 106 crates (103 without build tools) against 85 for `glfw` +
  `wgpu`. Accepted.
- `winit` is pinned to its stable 0.30 line; 0.31 is still a beta.
  Wayland is left out for V1 (the development machine runs X11) and can
  be added by enabling its feature.
- `alsa` (one main author, but the binding the Rust audio ecosystem
  builds on, and confined to the ALSA output adapter) is kept for V1 as a
  known single-author dependency. Its replacement boundary is that
  adapter.
- The smaller crates `winit` and `wgpu` pull in (for example `x11rb`,
  `x11-dl`, `xkbcommon-dl`, `gpu-allocator`) are inherited, not chosen;
  `Cargo.lock` pins them.

**D-10 Voice tracks and control tracks; the count-in is part of the
score.** Decided by Jim, 4 Nov 2026.

*Why.* The count-in and the beat clock were player behaviour outside the
score: a count-in lay before 0 ticks with no score positions. Each fix
added a player state, a control or an open decision (O-2, O-3, O-13).
Written into the score instead, they need no special handling: the
player plays exactly what is written (A-9), and the screen draws them
from what is published (A-5).

- **Two kinds of track.** *Voice tracks* are standard notation that
  becomes MIDI notes: harmony and bass in V1; drums and colour phrases
  later. *Control tracks* carry what this song needs from the studio's
  other devices.
- **Phrase cue.** A marker at a score position that sends plain MIDI: a
  note (on and off; a short note is a trigger, a held one a gate), a CC,
  a program change, or — on a control track only — transport (Start,
  Stop, Continue). A cue on a control track names a destination, which
  the studio configuration maps to an output port and channel (D-11). A
  cue aimed at a voice track goes to that track's own port and
  channel, carries channel messages only, and is drawn above the staff
  as a marking. Transport has no channel: a transport cue's destination
  is a port only. Cues are placed by writing a rhythm line (Song
  Notation), not by bar number.
- **Beat pulse.** An ordinary mono voice playing one fixed note on the
  elements of its rhythm, the accent written with `>` where it is wanted.
  It has no special case in the vocabulary or the schedule: every voice is
  a note-on with a velocity and a note-off at the end of the written
  length, which is the gate length at a CV and gate output (EFS-2.6). How
  the receiver makes signals of the note (FH-2 output mode, or the
  Disting NT directly) is the receiver's setting.
- **Count-in.** Authored bars at the start of the song, in a first
  section marked *count-in*: voices rest, the beat pulse plays. Play
  starts at P = 0 ticks, the first bar of the score, count-in included.
  A transport cue on the music's first bar starts the followers, so they
  never play the count-in. The player has no count-in state; its only
  count-in control is the practice one below.
- **Navigation.** A return that should skip the count-in is a D.S. to a
  segno on the music's first bar, as in standard notation. A cue can be
  limited to a pass ("1st time only"), as a marking can.
- **Output ports.** The program has several ALSA output ports, named for
  their purpose (e.g. `Clock`, `Harmony`, `SQ-1`) and declared in the
  studio configuration (D-11). Connecting them to
  devices is the studio's job — the MIDI router, `aconnect`, or anything
  else; the program does not depend on the router. **Exactly one port
  sends Timing Clock**, so the program is one clock source; each device
  that follows clock is connected to it once. A transport cue goes on
  one of two kinds of port. On `Clock`: a MIDI router that takes clock
  and transport only from one clock master (as Jim's does) then decides
  route by route which devices receive it. On a port of its own (e.g.
  `To SQ-1`), for direct connections (`aconnect`): a Start there reaches
  only what that port is connected to.
  Devices that share a port downstream (the disting NT behind the FH-2,
  a DIN thru chain) receive the same transport; control them one by one
  with channel messages (reset or gate notes on separate channels).
- **External phrases.** A sequencer's phrase length is stated as
  steps × step value (8 steps × an eighth = 20160 ticks), not in beats,
  so a cue's end ("2 runs from bar 5") is worked out once when the song
  is loaded. The player counts no beats while playing. Turning a named
  action ("SQ-1: pattern 3, 2 runs") into cues is a device profile's job,
  later; in V1, cues are written as plain messages.
- **Play from a section, and practice.** Playing from a section starts
  at the section's first bar, with no count-in. For practice, a play
  screen control, *count-in first* (on or off, kept until changed),
  plays the song's count-in section once before the first pass, then
  jumps to the section (a new anchor, as for the practice loop); the
  section then loops without a count-in until stopped. Every Play honours
  the control's setting. If the song has no count-in section, the
  control is unavailable and says so. At the start position the player
  sends the state in effect there — the last program change and CC
  values of each cue target, and any note held as a gate — then the cues
  from the section on. Devices are put in step by resets at
  phrase boundaries; Song Position Pointer is not assumed, because many
  devices ignore it. A section start that falls inside an external
  phrase is shown as a warning in the authoring view.

- **Pulse bursts (later).** A cue may start a burst of fast pulses for a
  set length: to clock a sequencer far above the song's tempo, so it
  becomes an oscillator (its gate a waveform), or to fire fast voices.
  The program sends Timing Clock on one port only, at the song's tempo,
  so a device cannot be given a faster MIDI clock of its own: a second
  clock reaching it adds to the first (EFS-8.4). A burst is therefore a
  stream of notes, like the beat pulse, at its own rate (e.g. 64ths),
  made into a clock by a gate (an FH-2 output) into the device's clock
  or SYNC input. Observed 4 Nov 2026 (`control_track_test`): the SQ-1 and
  a VPME.QD drum voice kept up with Timing Clock at about 22000 q/min,
  about 9500 events/s, once the output pool was large enough.

*Set aside:* the count-in and beat clock as player functions, with the
states, control and open decisions they needed; Song Position Pointer
and DAW-style chasing of transport for external devices; one output
port for everything (every Start would reach the whole studio); Timing
Clock on several ports (a device reached by two adds them, EFS-8.4).

**D-11 The song is a composition; the studio configuration realises it.**
Decided by Jim, 5 Nov 2026.

*Why.* The same song should play through different studios, and a
device's quirks (which octave it calls C0, how long its gate must drop to
retrigger, how long a quantizer needs to settle, how hard it responds to
an accent) belong to the device, not to the music. Ports and channels in
the song tied it to one studio.

- **Two kinds of file.** The *song* names tracks, with a type and a clef,
  and holds sections, lines, cues and (later) tempo marks; no port,
  channel, velocity number or device. The *studio configuration* declares
  the ports, and for each track of a song, by name: port, channel,
  octave offset, velocity levels and device timing; and maps each cue
  destination to a port and channel. One configuration serves many songs.
- **The configuration is not vocabulary.** A-1 keeps devices out of the
  vocabulary, so the configuration's type lives outside it; the parser
  for all the files is one adapter crate, `song_text` (A-3).
- **Track types** say what a track can do and so what the receiver must
  be able to do: poly voice, mono voice, clock and transport, cue
  control in V1 (a beat pulse is a mono voice); tempo control, musician voice and automation control
  later (Song Notation, section 5). Clock and transport has a port and no
  channel.
- **Pitch** is stored as the staff pitch in the track's clef; the
  octave offset in the configuration corrects the device (musician voices,
  later, store sounding pitch and draw transposed).
- **Marks** (accent, strong accent, staccato, legato, fermata) are written
  on elements; how hard an accent plays is the configuration's. A fermata
  never changes the clock in V1.
- **Cues and tempo marks are rhythm lines**, so authors count note values,
  not bar numbers across repeats.
- **A file with errors** does not load, not even partly, but the program
  starts and shows the errors; the last good song keeps playing.
- **Deferred:** polymeter (lines of different lengths in a section);
  tempo ramps, which need a design in Musical Time and Clock first (its
  anchors hold a constant tempo each).

*Set aside:* ports and channels in the song (the earlier draft); a
velocity per track in the song; a track-wide *no root* setting (a
voicing without the root is written note by note, and the editor can do
it for a whole line); bar numbers for cues (hard to calculate through
repeats); an independent play position per track (one bar position is
what the counter, cues, key wheel and strip all rely on).

## Development environment

The development machine: Linux Mint 22 (Ubuntu 24.04 base), Cinnamon on
X11, AMD Radeon RX 9070.

**System packages:**

```bash
sudo apt install build-essential pkg-config libasound2-dev \
     mesa-vulkan-drivers libvulkan1 vulkan-tools
```

None of these is Mint-specific; all are standard Linux layers. The only
Linux-specific part of *our* code is the ALSA MIDI output.

**Vulkan, checked 28 Oct 2026 with `vulkaninfo --summary`:**

- GPU0: AMD Radeon RX 9070, driver RADV, Mesa 25.2.8, Vulkan 1.4.
  RADV warns it is "not a conformant Vulkan implementation" — a
  certification notice for this recent GPU generation, not a fault.
- GPU1: llvmpipe, Mesa's CPU Vulkan — the automatic fallback when no GPU
  is usable. wgpu is asked for the high-performance adapter, so it picks
  the RX 9070.
- Vulkan loader 1.3.275; surface extensions for X11 and Wayland present.

**Window:** `winit` with default features off and `x11` enabled
explicitly. It loads the X11 libraries at run time, so it needs no
development packages and no `cmake` (checked 29 Oct 2026: it builds on
the development machine without installing anything). `glfw` was tried
first and set aside (D-9).

## Next

**Done so far (spike steps 1–4b).** The workspace and its strict lints;
musical time (D-7: `ScorePosition`, `ScoreLength`, `Instant`,
`Duration`); the ALSA output with the clock measurements (D-8); the
window, the three shaders and the glyph atlas; one voice track's strip with
chord symbols, slash rhythm line, hideable pitch staff, and a wheel that
follows the chords. The playhead still runs on the computer's own clock,
not the queue's.

**1. ALSA output for D-10.** Declare `queue_clock.rs` (the screen's
read-only handle on the queue's real time, D-8) and let
`SequencerOutput::now` use its `read_queue_instant` instead of its own
copy. Several named output ports on one queue. Add Start, Stop,
Continue, CC, program change, removing queued events, and Note Off sent
at once (Musical Time and Clock, sections 8 and 9).

**2. First tracks and cues.** The vocabulary's first types for voice
tracks, control tracks and phrase cues (D-10), and an `alsa_output`
example, `control_track_test`, that plays a hard-coded song at a fixed tempo:
a one-bar count-in section, a beat pulse on one port, Timing Clock on the
clock port, and a Start cue on the music's first bar on a port of its
own. Jim runs it:

1. The beat pulse to an FH-2 gate into the Quad Clock Divider, with a
   reset note to an FH-2 trigger output into its reset.
2. The Start cue on a port connected only to the SQ-1's `CTRL` port,
   through the MIDI router and through `aconnect`.
3. Directly to the disting NT and its own sequencers, with no FH-2.

What to watch: whether the divider is steady through the count-in and
on the beat at the music's first bar; whether the SQ-1 starts exactly on
that downbeat and nothing else starts; whether anything plays during
the count-in. Results go in the test's `explainer.md`
(`crates/alsa_output/examples/control_track_test/`).

*Status, 4 Nov 2026.* Done: the SQ-1 started, stopped and restarted on
cues through the MIDI router (`--transport-on-clock`), and the output was
stress-tested to 30000 q/min (output pool raised to 2000 events). Open,
for next session: Jim sets up the FH-2 outputs for the beat pulse and the
reset trigger (not as simple as a gate output); then the divider and
downbeat checks above, and the direct `aconnect` path.

**3. Spike step 5: timing and drawing together.** A sending thread in
`app` sending Timing Clock (24 pulses/q) and the control track, stamped
up to 50 ms ahead, instants computed from an anchor (section 6);
the playhead and wheel driven by the queue's real time instead of the
computer's clock; frame-time statistics printed on exit; then the
`delivery_spread` measurement repeated while the window draws.

**4. Song notation and the trial player.** Song Notation, Revision 8, is
the specification: song, studio configuration and libraries. Build order:
`rhythm_notation_test`; `song_play_test` (the player's schedule extended to
voice tracks, playing `Demo.song` through `Demo.studio`); `apply_test`;
then the editor with its text pane. The `retrigger_gap_test` spike can run
at any time. The aim is a working player, so the notation can be tried by
ear.

**Deferred.**

- *Key signatures and key changes* (a later spike): a song in a key other
  than C, with its key signature after each clef and chord names and
  voicings spelled by the key; a key change within the strip, drawn as
  in standard notation (EFS-1.9).
- *Layout options* (until the functions are complete). The spike's
  sizes are the legible minimum, so a crowded screen gains room by other
  means: a third row of chord symbols for the densest bars; n bars
  across in place of a fixed 8, or two lines of 4 bars stacked at a
  larger size; the wheel smaller in a corner, or left out when it cannot
  fit; a **Big** view for a screen shared by several musicians (e.g. a
  family jam on a large screen) that enlarges symbols and voicings for
  simpler songs, with the same fit check showing where a dense song no
  longer fits. Views showing tracks beyond the lead bring their own
  layout problems.
