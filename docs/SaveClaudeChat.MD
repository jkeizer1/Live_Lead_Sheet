# Saving a Claude session as readable text

Revision 1 — 6 Nov 2026

Why: a long session is compressed into a summary, and details are lost
(5 to 6 Nov 2026: the SQ-1 discussion, the clef rules, Jim's
song-structure example). The full record is kept by the app, but as raw
JSON. This page turns it into a readable conversation.

## What you get

A text file of Jim's messages and Claude's replies, in order, with
timestamps in UTC (six hours ahead of Jim's clock in November). Tool calls
are one line each; tool results, internal reasoning and system reminders
are left out. Pasted logs stay, so the file is long (the first one:
278 KB for 92 messages and 90 replies).

## Steps

1. Ask Claude: "export this session's transcript". Claude uses the app's
   export tool, which writes a zip into `~/Downloads` (about 4 MB) and
   gives its name, `session-export-<number>.zip`. The zip is only ever
   written to Downloads; nothing is uploaded.
2. Ask Claude to turn it into readable text, with the script below. Claude
   unzips it into its temporary folder, runs the script on
   `transcript.jsonl` and writes `session_transcript_<date>.md` in the
   project root. Jim moves it where he wants it.

Or by hand, from a terminal:

```bash
unzip -q ~/Downloads/session-export-NUMBER.zip -d /tmp/session_export
python3 render.py /tmp/session_export/transcript.jsonl session_transcript.md
```

## Things that went wrong the first time

- The zip was not yet in Downloads when Claude first looked. Look again
  after a few seconds.
- Claude cannot read `~/.claude` (the app's own copy of the session), so
  the export tool is the way to get the record.
- In that session Claude could read `~/Downloads` without asking, and
  could write only inside the project and its temporary folder. A later
  session may differ. If Claude is blocked, copy the zip into the project
  root yourself and say where it is.
- The app also unpacks a copy onto the Desktop. That copy is the app's;
  leave it.
- Never change `.gitignore` for the new file; Jim decides what is ignored.

## The script, `render.py`

Keep it here so it can be recreated; it needs only Python 3.

```python
"""Renders a Claude Code session export (JSONL) as a readable conversation:
the user's messages and the assistant's replies, in order. Tool calls are
shown as one line; tool results, thinking and system reminders are left out."""
import json, re, sys

src, dst = sys.argv[1], sys.argv[2]
REMINDER = re.compile(r"<system-reminder>.*?</system-reminder>", re.S)
out = []
seen = set()
counts = {"user": 0, "assistant": 0}

def text_of(content):
    if isinstance(content, str):
        return content
    parts = []
    for block in content:
        if block.get("type") == "text":
            parts.append(block.get("text", ""))
    return "\n".join(parts)

for line in open(src, encoding="utf-8"):
    try:
        rec = json.loads(line)
    except json.JSONDecodeError:
        continue
    kind = rec.get("type")
    if kind not in ("user", "assistant") or rec.get("isSidechain"):
        continue
    uid = rec.get("uuid")
    if uid in seen:
        continue
    seen.add(uid)
    msg = rec.get("message", {})
    content = msg.get("content", "")
    when = rec.get("timestamp", "")[:19].replace("T", " ")
    if kind == "user":
        text = REMINDER.sub("", text_of(content)).strip()
        if not text:
            continue
        counts["user"] += 1
        out.append(f"## Jim  ({when} UTC)\n\n{text}\n")
    else:
        pieces = []
        if isinstance(content, list):
            for block in content:
                if block.get("type") == "text" and block.get("text", "").strip():
                    pieces.append(block["text"].strip())
                elif block.get("type") == "tool_use":
                    pieces.append(f"*[tool: {block.get('name', '?')}]*")
        if not pieces:
            continue
        # a reply is several records; merge consecutive assistant records
        body = "\n\n".join(pieces)
        if out and out[-1].startswith("## Claude") and out[-1].endswith("\x00"):
            out[-1] = out[-1][:-1] + "\n" + body + "\n\x00"
        else:
            counts["assistant"] += 1
            out.append(f"## Claude  ({when} UTC)\n\n{body}\n\x00")
        continue
    # a user record closes any open assistant run
    if len(out) > 1 and out[-2].endswith("\x00"):
        out[-2] = out[-2][:-1]
text = "\n".join(o.rstrip("\x00") for o in out)
open(dst, "w", encoding="utf-8").write(
    "# Live Lead Sheet — session transcript\n\nExported from a Claude Code "
    "session. Jim's messages and Claude's replies, in order. Tool calls are "
    "shown as one line; tool results, internal reasoning and system "
    "reminders are left out.\n\n---\n\n" + text
)
print(counts, len(text))
```

## Revisions

- 6 Nov 2026, revision 1: first version, from the export of the session
  "Lead-sheet-bass MIDI output test".
