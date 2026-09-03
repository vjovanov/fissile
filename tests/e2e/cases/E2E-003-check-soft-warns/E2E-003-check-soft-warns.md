# E2E-003-check-soft-warns: a soft overflow warns without blocking

A file strictly above its soft limit but below the hard limit emits the warning
on stdout and still exits zero; equality passes. This is the agent-minimize half of the graded model
(§GOAL-006-graded-limits.1) — friction without a block — over `check`
(§FS-004-check-audit.1).

The detail line names no ceiling. Under the default 100-line step the file's
would be 100, and this rule's hard limit is 4, so a soft entry there would never
fire and `exception add` refuses to write one
(§DF-010-stated-ceilings-are-exact.2, §FS-004-check-audit.1).
