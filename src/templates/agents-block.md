## Keeping Files Small With fissile (v1)

This repository uses [`fissile`](https://github.com/vjovanov/fissile) to keep
files small so agents spend fewer tokens reading them, while respecting the
architecture. It is a simple guard, not a style police.

- Run `fissile check --staged` before claiming work is done.
- Treat a **soft** overflow as actionable guidance when you changed the file:
  split it the way the message suggests.
- Treat a **hard** overflow as stop-the-line: do not commit unless a structured
  exception already accepts the file.
- Read the message ID and guidance line as the configured remediation guidance
  for that rule.
- Record accepted agent-facing warning debt with
  `fissile exception add --severity soft`.
- Record human-reviewed blocking debt with
  `fissile exception add --severity hard` only.
- Run `fissile audit --stale-exceptions` before removing or moving large files.
