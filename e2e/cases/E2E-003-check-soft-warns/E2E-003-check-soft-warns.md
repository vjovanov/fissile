# E2E-003-check-soft-warns: a soft overflow warns without blocking

A file at or above its soft limit but below the hard limit emits the warning on
stdout and still exits zero. This is the agent-minimize half of the graded model
(§GOAL-006-graded-limits.1) — friction without a block — over `check`
(§FS-004-check-audit.1).
