# E2E-009-check-token-rule: a non-default config with a token-unit rule passes

A single config exercises the configurable surface together (§GOAL-005-configurable.4):
a custom per-extension line limit for `.rs`, a glob exclusion that drops a
generated `*.gen.txt` file from the scan, and a token-unit rule for `.txt` whose
overflow carries a custom message. The token count comes from an opt-in external
command (§DA-001-token-external-command) — here a POSIX `wc -w` stub — so the
case is Unix-gated; token mode itself is cross-platform. The `.txt` file is over
its token hard limit and reports with the token-unit finding shape over `check`
(§FS-004-check-audit.1), while the excluded generated file and the under-limit
`.rs` file stay silent.
