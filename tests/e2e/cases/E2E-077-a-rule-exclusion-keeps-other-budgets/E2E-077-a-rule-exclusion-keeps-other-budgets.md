# E2E-077-a-rule-exclusion-keeps-other-budgets: one rule can exclude a path without removing its other budgets

The changelog exceeds both the citable-spec line budget and the repository byte
catch-all. The line rule excludes that path, so it emits no line finding and
does not participate in rule selection, while the byte rule remains applicable
and emits its finding (§FS-001-config.3.4).

This is the distinction neither `[scan].exclude` nor an exception can express:
the file stays inside fissile's budget system, and no overflow is accepted
(§FS-001-config.3.3).
