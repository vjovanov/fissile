# E2E-015-check-groups-findings: findings group under one copy of their guidance

Three files over one rule produce two blocks, not five two-line findings: a hard
block and a soft block, each printing its own guidance once and listing the files
it applies to, largest first (§FS-004-check-audit.1). The rule carries a
different message per severity — *must split* names the human to escalate to,
*should split* names the exception that records the debt
(§DF-003-severity-guidance.1) — and `{rule}` renders inside a grouped message
because it is constant across the block (§FS-001-config.4). Guidance wraps at a
fixed width, so the same block is byte-identical in a narrow terminal and in CI
(§GOAL-006-graded-limits.2).
