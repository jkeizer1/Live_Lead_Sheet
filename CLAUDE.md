# Live Lead Sheet — working notes for Claude

A live lead sheet for improvising: it plays a song (chords, rhythm, bass)
to MIDI instruments and shows key, meter, chord names and voicings on a
staff, with the bars ahead in view. Owner: Jim.

## Read first

- `docs/Concept/Concept Brief.md` — what and why.
- `docs/Specifications/Experience Functional Specification.md` — the
  numbered requirements (EFS-n.m). V1 is the leader only.
- `docs/Architecture/Architecture.md` — principles (A-n), parts,
  technology, decision record (D-n), development environment, next steps.

When a requirement or decision is unclear, ask Jim. Do not invent
behaviour the documents don't state.

## Focus

V1 only: the leader, one native screen, Linux. Chains, other platforms,
browser and network clients are future work — keep the code from
blocking them (render crate takes only a surface and the stream; platform
code only in adapters), but do not build them.

## Working agreement

- **Jim commits and pushes.** Never run `git commit`, `git push`,
  `git reset`, `git checkout` of files, `git rebase` or `git stash`. Read-only
  git (`status`, `diff`, `log`) is fine.
- **Never use `sudo` or install system packages.** If a package is
  needed, stop and give Jim the exact `apt` command, so he can take a
  Timeshift snapshot first and run it himself.
- **Never add a crate without asking.** The allow-list is in
  Architecture.md ("Technology — V1"). Propose a crate with its purpose,
  its `cargo tree` count, and the alternative of writing the piece
  ourselves.
- **Stay inside this folder.** Do not read, write, move or delete
  anything outside the project, except what `cargo` itself needs.
- **Jim runs the program.** Build and run tests yourself; when the
  windowed program or MIDI output must be tried, tell Jim the exact
  command (`cargo run ...`) and what to look for.
- Work in small steps; say what you are about to change before changing it.
- Never modify `.gitignore`; ask Jim for new ignore rules. Use
  `--vcs none` with `cargo new`/`cargo init`.
- External crate versions live only in `[workspace.dependencies]` in the
  root `Cargo.toml`, written as compatible versions (e.g. `"0.12.1"`, not
  `"=0.12.1"`); `Cargo.lock` is the exact pin and is committed. Crates use
  `{ workspace = true }`.

## Coding rules

- Strict: Rust warnings as errors; `#![deny(clippy::all, clippy::pedantic)]`
  unless a specific lint is justified in a comment.
- No silent failures. Every error is handled; unrecoverable ones are
  reported with full context (what was being done, with what input), not
  only the failing call. No `unwrap()` or `expect()` outside tests.
- Never assign a default when a value can't be determined, unless it is
  truly harmless — and then say why in a comment.
- Explicit types on `let` bindings and function signatures. No
  macro or trait metaprogramming for cleverness; ordinary generics are
  fine.
- Concrete, purpose-named types — one type per musical or technical
  concept (`Chord`, `RhythmPattern`, `BarPosition`) — never generic bags
  or string-keyed maps standing in for a type.
- Justify every dependency against writing a small, one-off piece.
- Comments explain why, briefly. Jim reads code carefully; keep it plain.
- `lib.rs` holds only module declarations and the crate's doc comment;
  all code lives in purpose-named module files.
- Measurement and test programs go in the crate's `examples/` folder;
  throwaway experiments go in `/scratch`.
- Measurement programs (frame time, timing spread) run with `--release`;
  tests run in debug.

## Messages to the musician

Written for someone mid-session, in a loud studio at 3 AM: short, plain,
saying exactly what to change or type.
Example: `FH-2 gone. Plug it back in; it rejoins on the next bar.`
No sound alerts, ever.

## Look

Dark theme with soft contrast everywhere, including every control — no
bright white fields or dropdowns.

## Documents

- Stable filenames; the revision number lives inside each document
  ("Revision n — date"). No version numbers in filenames, no obsolete
  folders — git keeps history.
- Short and focused. Conceptual docs stay free of design detail and are
  written in generic roles, with Jim's setup as the example.
- Explanations target Jim's background: C, older C++, Java, ABAP,
  enterprise integration and architecture. He is learning Rust — explain
  Rust-specific idioms the first time they appear.

## Commands

To be filled in after the spike (build, test, run).
