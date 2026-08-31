# E2E-048-a-glob-ceiling-is-the-number-stated: a class-wide ceiling is the policy number chosen

A glob entry has no single file to measure, so it always takes `--max`
(§FS-005-exception-add.2). Under the old rule that number was then rounded to
the step, and a glob ceiling could never be the round policy number someone
picked — the entry §FS-005-exception-add.3 shows, `300000` bytes, was
unwritable under a 4096-byte step.

`--max 150` on `src/**` records 150, not the 200 the default 100-line step
would have chosen (§DF-010-stated-ceilings-are-exact.1). The same repository as
`E2E-032-bump-defaults-to-the-configured-step`, where the measured form of the
same command writes 200 for a 105-line file: the step is still in force, for
measurements.
