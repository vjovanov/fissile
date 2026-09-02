# E2E-063-remove-dry-run-writes-nothing: a dry run says what it would delete

`--dry-run` prints the entry and the registry it would update, and writes
nothing (§FS-009-exception-remove.4) — the same contract `add` and `retune`
carry. The scenario asserts the registry still holds the entry afterwards,
because stdout saying "would" is a claim and the bytes are the fact.
