# E2E-024-retune-preserves-the-registry: retune moves one line and reads the rest as TOML

The registry holds two entries. The first one's `reason` is prose that quotes a
registry — a `[[exceptions]]` header and a `max_accepted` line, inside a `'''`
literal string, under a comment that quotes `"""` — and the whole file is stored
with CRLF endings.

`fissile exception retune` finds the second entry, not the first: a string body
is prose, so nothing written there names an entry or shifts the block index, and
neither multi-line form nor a comment can open a fence the scan believes. The
one line it rewrites keeps the `\r` the file is stored with, so the diff is the
single decision that changed and not the whole file
(§FS-008-exception-retune.3).
