# E2E-066-audit-reports-a-ceiling-with-no-headroom: a ceiling sitting exactly on its file is a finding

An exact-path entry accepting precisely what its file measures grants no
headroom: it silences the finding today and stops on the first unrelated commit
(§FS-003-exceptions.7). `check` is quiet, because the exception matches the
measurement exactly, and `fissile measure` reports the same condition as
`0 to soft-accepted` (§FS-007-measure.2) — so the audit is the surface that has
to say it.

`audit --stale-exceptions` reports the entry in the `loose ceilings:` section,
named by its registry and path, with what it accepts, what the file measures
now, and the ceiling that grants headroom again: the step's next multiple
strictly above the one recorded (§FS-004-check-audit.2). It stays a report —
the run still exits `0`.
