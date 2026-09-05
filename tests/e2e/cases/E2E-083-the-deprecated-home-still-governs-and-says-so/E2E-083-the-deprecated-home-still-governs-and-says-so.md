# E2E-083-the-deprecated-home-still-governs-and-says-so: the old path keeps working, and says to move

A repository that has not migrated is not broken by the move. Its
`.agents/fissile.toml` is still discovered, one step behind the new home, and
still governs the run (§FS-001-config.8.1). Reading it does not change the exit
code: `limits` answers `0` here exactly as it would from either path.

What is new is that the run says the path is deprecated and where to move it,
once, on stderr (§FS-001-config.8.2). Without that line the fallback is
permanent — every run passes, nothing is ever migrated, and the read-only
`.agents/` this move exists to escape keeps holding the file
(§DF-012-config-home).

The stdout assertion is exact, which is how the scenario proves the negative
half of the same rule. Stdout carries the findings and, under `--format json`, a
stream a caller parses; a deprecation notice that reached it would break that
caller for a reason that has nothing to do with the caller's tree.
