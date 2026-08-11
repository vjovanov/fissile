## Keeping Files Small With fissile (v1)

This repository uses [`fissile`](https://github.com/vjovanov/fissile) to keep
files small so agents spend fewer tokens reading them, while respecting the
architecture. It is a simple guard, not a style police.

- Run `fissile check --staged` before claiming work is done.
- Findings are grouped: the block header names the severity, rule, and limit,
  and the guidance under it is this repository's configured remediation for
  every file listed below it.
- A **soft** overflow means *should split*: if you changed the file, split it
  the way the guidance says — along a seam that already exists, never at the
  line count, never breaking apart code that belongs together.
- A **hard** overflow means *must split*: stop the line. Do not commit unless a
  structured exception already accepts the file.
- Never damage the design to fit a budget. If no split leaves the code better,
  record a soft overflow with `fissile exception add --severity soft`; for a
  hard overflow, ask a human — `--severity hard` is theirs to add, not yours.
- An exception's `--reason` is a claim, not a description of the file. Say
  either what makes splitting illegal (`--kind structural`, never expires) or
  which boundary is missing and what has to exist first (`--kind deferred`,
  with `--until` naming what retires it). Restating the finding is not a reason.
- Run `fissile audit --stale-exceptions` before removing or moving large files.
