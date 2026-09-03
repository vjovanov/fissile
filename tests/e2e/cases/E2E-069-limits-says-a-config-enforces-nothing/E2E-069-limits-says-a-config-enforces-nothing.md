# E2E-069-limits-says-a-config-enforces-nothing: an empty inventory is an answer

A config with no `[[rules]]` is valid and enforces nothing. Printing nothing for
it would be indistinguishable from a command that failed quietly, or from one
whose output was swallowed, so `limits` says `no rules configured` and exits `0`
(§FS-010-limits.2) — the reason `measure` names a file no rule measures rather
than dropping it (§FS-007-measure.2).
