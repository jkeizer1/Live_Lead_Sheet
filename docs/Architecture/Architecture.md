# Live Lead Sheet — Architecture

Revision 12 — 29 Oct 2026

Builds on the Concept Brief (Revision 8) and the Experience Functional
Specification (Revision 5). Sets the principles, the main parts, how
they talk, and the technology for V1.

**Focus.** V1 is the Live Lead Sheet itself: the leader, on one native
screen, on Linux. Everything below about chains, other platforms and
networks is there so V1 does not block it — not to be built now.

## Principles

**A-1 The vocabulary is the core.** At the centre is a symbolic musical
language: key, meter, tempo, chord, voicing, rhythm pattern, part,
section, song, position, trigger. Everything else is built around it. It
represents music as a musician thinks of it, not as any device or file
encodes it.

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
trigger — as they happen. A follower knows what is coming by combining
the two; the future is never streamed.

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
- **Chord progression catalog** — progressions in Roman numerals.
- **Rhythm pattern catalog** — one- to eight-bar patterns.

All three hold the vocabulary, stored as text files through a file
adapter (A-3).

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
               (e.g. melody   (e.g. other        notes, clock,
                generator,     generative,       beat notes,
                own view)      own view)         triggers
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
| player | reading the clock, publishing (A-5 – A-7) |
| render | layout (vocabulary → instance records) and GPU drawing |
| alsa output | the MIDI output (A-3), and the clock it supplies (A-7, D-8) |
| app | windows, authoring, wiring the parts together |

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
(rhythm patterns assigned to bars, lead part, *no root*, triggers as
actions, the tempo slider). MIDI 2.0's Flex Data messages (chord name,
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

The full decision — reading the clock, the timer evidence, re-anchoring,
loop release and stop — is in sections 5 and 8 of
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

1. **Spike:** at 2560 × 1440, draw an 8-bar strip (chord symbols, rhythm
   line, staff) and the wheel with the three shaders, with a moving
   playhead; alongside, an ALSA thread sending MIDI clock and beat
   notes. Measure frame time and clock evenness.
2. **Song notation:** first draft, worked through on a 12-bar blues.
3. **Exact musical time:** settled — D-7.
