# Working with Claude Code

Revision 2 — 28 Oct 2026

How Claude Code is set up for this project, what protects the rest of the
computer, and what doesn't. Based on the Claude Code documentation as of
October 2026 (links at the end). Check them again before relying on a
detail — the product changes often.

## The one idea to hold on to

There are two layers, and only one of them is a guard:

| Layer | File | What it does |
|---|---|---|
| **Instructions** | `CLAUDE.md` | Tells Claude how to work. It shapes what Claude *tries* to do. It enforces nothing. |
| **Enforcement** | `.claude/settings.json` (and `~/.claude/settings.json`) | Permission rules, enforced by the Claude Code program itself; and the **sandbox**, enforced by the Linux kernel. |

In the documentation's words: "Permission rules are enforced by Claude
Code, not by the model. Instructions in your prompt or `CLAUDE.md` shape
what Claude tries to do, but they don't change what Claude Code allows."

So every protection that matters is in a settings file; `CLAUDE.md`
repeats them only so Claude doesn't waste time trying.

## The three protections, from outside in

**1. The sandbox (strongest).** Every shell command Claude runs — and
everything that command starts, such as `cargo` and the build scripts of
crates — runs inside `bubblewrap`, a Linux user-namespace sandbox. The
kernel enforces:

- **Writes** only inside this project folder, the per-user temp
  directory, and the two cargo download caches (`~/.cargo/registry`,
  `~/.cargo/git`). Everything else in the home folder and the system is
  read-only to the command — it cannot delete, move or rename anything
  outside.
- **Reads** blocked for the private folders, named one by one: `~/.ssh`,
  `~/.gnupg`, `~/.config`, the keyring, `~/.mozilla`,
  `~/.git-credentials`, Claude Code's own `~/.claude` and `~/.claude.json`,
  and `~/.bash_history`. The sandbox's default is to allow reading the
  whole computer; this list narrows it. (Blocking the whole home folder
  and re-allowing the toolchain did not work — see "Lessons" below.)
- **Network** only through Claude Code's proxy, and only to crates.io.
- If the sandbox can't start, Claude Code exits rather than running
  unprotected (`failIfUnavailable`), and Claude can't ask to run a
  command outside it (`allowUnsandboxedCommands: false`).

**2. Permission rules and Manual mode.** Claude Code asks before every
shell command and every file edit; you see the exact command or change
and answer yes or no. Some things are refused outright: `sudo`, `su`,
`apt`, `dpkg`, `snap`, `flatpak`, `cargo install`, `curl`, `wget`, git
commands that change history or remotes, and reading `~/.ssh`,
`~/.gnupg`, `~/.config`, the keyring and browser profiles. Claude's own
file-reading tools ask before reading outside the project, and refuse the
private folders above, plus `~/.claude`, `~/.claude.json` and
`~/.bash_history`. Bypass mode is disabled.

**3. Your habits.** The prompts only protect you if you read them — see
"Each session" below.

## Your questions, answered

**Can it be kept to this folder?** Yes, for writing: the sandbox blocks
shell commands from writing outside it, and Claude's edit tool asks
before touching any file. For reading: the private folders are blocked;
other files in your home folder and system files (such as `/etc`) are
readable, as for any program you run — but every command is shown to you
before it runs.

**Can it be stopped from deleting, moving or renaming outside the
project?** Yes — at kernel level for shell commands (sandbox), and by
prompt for Claude's own file tools. Inside the project, `rm` and `mv`
still ask.

**Can it be stopped from installing packages without a Timeshift
snapshot first?** Yes. Claude cannot run `sudo` or any package manager —
refused by rule, and `sudo` cannot gain privileges inside the sandbox
anyway. When a package is needed, Claude stops and gives you the
command. You take the snapshot, then run it yourself.

**Can it be stopped from starting something harmful?** Partly — and
honestly, this is the real risk. Building Rust code runs code from the
crates: each crate's build script and procedural macros execute on your
machine during `cargo build`. Our defences:

- the crate allow-list (Architecture.md) — few crates, each justified;
- the sandbox — that code can't write outside the project and the cargo
  caches, can't read your home folder, and can reach only crates.io;
- you run the finished program yourself, outside Claude Code.

**What is *not* protected?**

- The sandbox covers shell commands only. Hooks, MCP servers and plugins
  run with your full access. **This project uses none; don't add any
  without thinking it through.**
- Rules that match command text (like `Bash(curl *)`) can be sidestepped
  by writing the command differently (`/usr/bin/curl`). The sandbox is
  what actually holds; the text rules catch the ordinary case.
- "Yes, and don't ask again" saves a permanent rule in
  `.claude/settings.local.json`. Read that file from time to time.
- Anything Claude reads is sent to Anthropic to be processed. Blocking
  reads is also about privacy.
- The documentation says plainly: "Sandboxing reduces risk but is not a
  complete isolation boundary." For a harder boundary, run Claude Code as
  a separate Linux user that owns only this folder, or in a virtual
  machine.

## The files

| File | Scope | In git? |
|---|---|---|
| `~/.claude/settings.json` | you, every project | no |
| `.claude/settings.json` | this project — the rules above | yes |
| `.claude/settings.local.json` | your saved "don't ask again" answers | no (ignored) |
| `CLAUDE.md` | instructions for Claude in this project | yes |

A deny rule anywhere wins over an allow rule anywhere.

## Setting up, in order

1. **Back up first.** Take a Timeshift snapshot with *hidden files in
   home* included (Timeshift → Settings → Users), so `~/.config`,
   `~/.cargo` and the other parts of your home environment are covered —
   your current backups cover data only. Run your data backup too.
2. **Install the sandbox's tools:**
   `sudo apt install bubblewrap socat`
3. **Check AppArmor.** Ubuntu 24.04 (Mint 22's base) can stop
   `bubblewrap` from working. Run
   `sysctl kernel.apparmor_restrict_unprivileged_userns` —
   if it prints `1`, follow the AppArmor step on the sandboxing page
   before going on.
4. **Install Claude Code** from the official setup page (link below).
5. **Your own settings** — create `~/.claude/settings.json`:

   ```json
   {
     "permissions": {
       "defaultMode": "default",
       "disableBypassPermissionsMode": "disable",
       "disableAutoMode": "disable",
       "deny": ["Read(~/.ssh/**)", "Read(~/.gnupg/**)", "Bash(sudo *)"]
     }
   }
   ```

   This matters: recent versions *start in auto mode*, where a second
   model approves actions instead of you. This file makes Manual mode
   the start and removes auto and bypass. (If Claude Code reports
   `disableAutoMode` as unknown here, it belongs at the top level instead
   of inside `permissions` — the docs show both.)
6. **First start:** `cd ~/Documents/Live_Lead_Sheet`, then `claude`.
   Accept the folder-trust question only after reading what it lists.
   Then:
   - `/permissions` — check the rules shown match `.claude/settings.json`;
   - `/sandbox` — check it says enabled, with no missing dependencies;
   - `claude doctor` (in a terminal) — check for settings warnings.
7. **First task — check `cargo` works inside the sandbox.** If it fails
   writing `~/.cargo/.package-cache`, add that path to `allowWrite`.
   Settings are read at start-up: after any change, `/exit` and start
   `claude` again.

## Each session

- Start with a clean `git status` — commit first. Then `git diff` shows
  exactly what Claude changed, and `git restore` undoes it.
- Stay in Manual mode. **Shift+Tab** cycles modes — check the label at
  the bottom of the screen if you pressed it by accident.
- Read each prompt. **Esc** stops Claude mid-action.
- Use "Yes, and don't ask again" only for harmless commands
  (`cargo build`, `cargo test`).
- Before any package install: Timeshift, then run the command yourself.

## Lessons from the first session (28 Oct 2026)

- **`blockReadsOutsideWorkingDirectories: true` also fences the
  sandbox.** Permission rules and sandbox settings are merged, so this
  setting hid the whole home folder — including `~/.cargo` and `~/.rustup`
  — from shell commands, and no `allowRead` exception (with `~` or full
  paths) got through. `cargo` was "not found". Fix: set it to `false`,
  and deny the private folders by name instead, in both the sandbox's
  `denyRead` and the permission `Read(...)` deny rules.
- **A new Claude session does not remember earlier ones.** It may
  suggest a fix that was already tried. Settings decisions are recorded
  here, not in the chat.
- **Edit the guard yourself.** Decline Claude's offers (including the
  `update-config` skill) to change `.claude/settings.json`; make the
  change in your editor, check it with `claude doctor` or
  `python3 -m json.tool`, and restart.
- **`!` at the prompt runs a command outside the sandbox.** Useful for
  diagnosis; use it deliberately.

## Sources

- [Security](https://code.claude.com/docs/en/security)
- [Configure permissions](https://code.claude.com/docs/en/permissions)
- [Permission modes](https://code.claude.com/docs/en/permission-modes)
- [Sandboxing](https://code.claude.com/docs/en/sandboxing)
- [All settings](https://code.claude.com/docs/en/settings-reference)
- [Setup](https://code.claude.com/docs/en/setup)
- [Terminal guide for new users](https://code.claude.com/docs/en/terminal-guide)
