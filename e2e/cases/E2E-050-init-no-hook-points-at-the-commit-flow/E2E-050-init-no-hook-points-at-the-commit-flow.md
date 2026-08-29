# E2E-050-init-no-hook-points-at-the-commit-flow: a declined hook sends the reader to their own commit flow

`--no-hook` suppresses the automatic pre-commit install (§FS-002-init.6), and
the `next:` block must not promise machinery the run did not install
(§FS-002-init.5). This runs inside a git repository, where the hook would
otherwise have been written, so nothing but the flag stops it:
`.git/hooks/pre-commit` stays absent, and step 2 names the flag that skipped it
and the wiring left to do — without asserting a hook manager `init` never looked
for, since the reader may have one or may simply not want a hook
(§GOAL-003-friendly-output).
