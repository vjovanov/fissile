# E2E-088-audit-only-keeps-a-failing-exit: hiding a finding does not unfail the run

`fissile audit` exits non-zero for a standing hard overflow
(§FS-004-check-audit.2). `--only` selects what is printed and nothing else, so a
repository that fails still fails when the failing section is not on screen.

This is the boundary most worth an executable proof. Everything else `--only`
gets wrong is visible in the output a caller is reading; an exit code that
followed the selection would be invisible until a gate let a hard overflow
through, and by then the flag would be in every script that hid it.

The same holds for the exit `2` a file that could not be measured produces, and
for the stderr diagnostics either failure carries (§FS-004-check-audit.5): they
are on stderr, which `--only` does not reach.
