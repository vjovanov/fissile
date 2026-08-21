# E2E-032-bump-defaults-to-the-configured-step: a ceiling is quantized even when nothing configures the step

`[exceptions.bump]` decides every ceiling `fissile` writes, and its defaults —
100 lines, 4096 bytes, 1000 tokens (§FS-001-config.5) — are what a repository
that never mentions the table gets. This config never mentions it.

A 105-line file is accepted at 200: the smallest multiple of the step at or above
the measurement, so the registry records a decision rather than the reading taken
on the day the entry was written (§FS-005-exception-add.2,
§DF-006-quantized-ceilings.1). Were the default to drift, or the field to become
optional-per-field, every ceiling the tool writes would change and the change
would be invisible.
