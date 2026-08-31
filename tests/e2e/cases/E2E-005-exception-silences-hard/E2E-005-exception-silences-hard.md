# E2E-005-exception-silences-hard: a hard exception accepts the file but the soft warning survives

A hard-registry exception sized at or above the file silences the blocking
finding, so `fissile check` exits zero — but the soft warning still appears so an
agent keeps minimizing accepted debt. This is the registry override
(§GOAL-007-justified-exceptions) and the deferred half of the
silenced-hard-and-the-soft-finding rule (§FS-003-exceptions.3); the structural
half is `E2E-019-structural-silences-soft`.

The entry declares no `kind`, so it reads as `deferred` (§FS-003-exceptions.2.1):
a registry written before the field existed keeps the behavior it had.
