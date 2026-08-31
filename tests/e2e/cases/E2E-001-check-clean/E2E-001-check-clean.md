# E2E-001-check-clean: a clean check prints `ok` and exits zero

A file under every budget produces no findings. `fissile check` prints exactly
the success marker on stdout and exits zero — the explicit-success contract
(§GOAL-003-friendly-output.1) and the clean state of the graded model
(§GOAL-006-graded-limits.4) over the `check` surface (§FS-004-check-audit.1).
