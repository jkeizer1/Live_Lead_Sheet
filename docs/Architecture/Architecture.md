# Live Lead Sheet — Architecture

Revision 4 — 28 Oct 2026

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
hardware or screen: the MIDI sender and the display. One clock, owned by
the player, sets the tempo for everything.

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
| `glfw` (default features off) | window and input |
| `wgpu` (pinned) | GPU drawing |

Measured: about 70 crates in all, almost all of them wgpu's. wgpu changes
its major version every few months; it is pinned and upgraded only by
choice.

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
| vocabulary | the musical language (A-1, A-2) |
| player | musical time, the clock, publishing (A-5 – A-7) |
| render | layout (vocabulary → instance records) and GPU drawing |
| alsa output | the MIDI output (A-3) |
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

## Next

1. **Spike:** at 2560 × 1440, draw an 8-bar strip (chord symbols, rhythm
   line, staff) and the wheel with the three shaders, with a moving
   playhead; alongside, an ALSA thread sending MIDI clock and beat
   notes. Measure frame time and clock evenness.
2. **Song notation:** first draft, worked through on a 12-bar blues.
3. **Exact musical time:** its representation, settled in the spike.
