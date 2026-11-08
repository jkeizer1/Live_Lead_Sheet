# Live Lead Sheet — Concept Idea: Decoupled Composition for a Hybrid Studio

Revision 1 — 8 Nov 2026

A standalone note of intent. It sits beside the Concept Brief and does not
replace it. It records an emphasis, not a design: what is open is marked as
open.

## 1. The point

Standard notation couples everything about a sounding element into one
symbol: its pitch comes from where it sits on the staff and in which clef,
its duration from the note's shape, its articulation and dynamics from marks
around it. Score software, and the MIDI note that follows it, inherit that
coupling: one note-on carries pitch, start, length and loudness together.

A modular instrument does not work that way. One sequencer generates gates,
another generates pitches, and both feed the same voice through separate
inputs. A third source can shape the timbre: for example, a sequencer
producing impulses that drive the envelope of a waveshaper. Pitch, rhythm and
timbre are independent streams, combined only at the voice. That **low
coupling** is what makes the modular flexible and alive, and it is the
character this software exists to serve.

## 2. Two capabilities, in order

1. **A basic standard-notation composer and player.** Enough to write and
   play songs in standard notation: answer the open questions, and accept
   limitations rather than match mature notation software. Bars that add up,
   voice layers, ties, keys and meters belong here. This is what the name
   *Lead Sheet* promises: the essential harmonic changes laid out over time.
2. **Decoupled, modular-style shaping of that music.** The capability that
   makes the software distinct, and where the main effort should go once the
   first is good enough.

A standard synthesizer (a monologue, minilogue or Mother-32, for example)
is built for the coupled MIDI stream, and the first capability already
serves it. The second capability is for the modular and for the hybrid
studio as a whole.

## 3. Composition at the macro level

Standard music software assumes the composer works at the finest level of
detail, every sixteenth, accent and slur considered, as Debussy did.

The intent here is composition one level up. The libraries hold building
blocks, chord progressions and rhythm patterns, each a phrase rather than a
note. They are assembled into a song structure: sections, repeats, keys,
tempos. That structure is the **substrate** the modular world runs on.

Harmonic movement over time is exactly what is hard to get from the modular
alone; short loop sequencers (the familiar complaints about 16-step
sequencers) do not carry chord changes across a song. The song provides the
changes; the modular provides the life.

The complete-bar rule comes from standard notation and stays: it is what
makes the harmonic structure reliable. Decoupled shaping is laid over that
structure; it does not loosen it.

## 4. Decorating sounding elements

As a track can be split into voice layers, a track (the lead track or any
other) can carry several supporting **functional or decorating** streams:
rhythm shaping, timbre shaping, envelope or modulation shaping, each realised
in the studio's hardware rather than in the note.

So far the song only knows the basics: a cue can trigger something, but there
is no symbolic language for the *intent* of what should happen in the modular,
and no way to drive the modular directly from the score. Cue tracks, and the
discrete-event and continuous automation track types discussed so far, are a
small first hint of this.

The composition as a whole is then modular in nature: notes, rhythms and
timbre each decoupled, attached and recombined.

Open: what these streams are, how they are named, how they attach to a
track or to its elements, and how intent is written so that it is neither a
device setting nor an engraving mark.

## 5. Views

The composer shows standard notation first. The decoupled, modular views,
showing what shapes each sounding element and where, are what set this
software apart from using a notation program (GuitarPro, MuseScore) to send
MIDI or MusicXML to a standard player.

Open: what those views look like.

## 6. Signals: MIDI so far, DC-coupled later

Everything to date goes out as MIDI, through interfaces such as a MIDI-to-CV
converter. That has been a reasonable start. Full use of DC-coupled outputs
(control voltages produced directly by the computer's interface) has not yet
been considered, and is likely to matter for the decoupled streams of
section 4.

Open: whether, and how, the song drives DC-coupled outputs, and how this
stays separate from the studio configuration (the song says *what*, the
studio says *where*).

## 7. What stays out of the song file

Engraving choices (which rests are hidden, how voices are drawn) are not
in the song file. Several engravers could exist later, as optional plug-ins;
that is out of scope now.
