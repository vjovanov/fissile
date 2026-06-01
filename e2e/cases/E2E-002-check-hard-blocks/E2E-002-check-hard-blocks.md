# E2E-002-check-hard-blocks: a hard overflow fails the commit with a named fix

A file at or above its hard limit makes `fissile check` exit non-zero and print
the byte-stable finding line plus the configured guidance line — the
stop-the-line half of the graded model (§GOAL-006-graded-limits.1) carrying the
remediation message (§GOAL-008-remediation-messages) over `check`
(§FS-004-check-audit.1).
