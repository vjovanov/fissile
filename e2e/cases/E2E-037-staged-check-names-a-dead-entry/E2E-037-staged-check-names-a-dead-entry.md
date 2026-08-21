# E2E-037-staged-check-names-a-dead-entry: a commit blocked by a dead entry says so

Under `[exceptions].stale = "error"` a leftover exception fails the run
(§FS-003-exceptions.4), and under `--staged` that run is a commit. The epilogue
that closes it is chosen by what blocked it (§FS-004-check-audit.1.2): there is
no file to split here, so the gate names the registry instead.

The case also pins what makes the entry dead. The commit stages the removal of
`src/moved.rs`, which is the file set proving the entry has outlived its file
(§FS-004-check-audit.1.3) — not the mere absence of a path from the working
tree, which §E2E-038-an-unbuilt-file-is-not-a-dead-entry holds separately.
