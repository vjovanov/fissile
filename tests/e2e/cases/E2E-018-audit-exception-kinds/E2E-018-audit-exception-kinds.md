# E2E-018-audit-exception-kinds: audit separates accepted-permanently from carrying-debt

`audit` counts the exception registries by kind rather than reporting one total
(§FS-004-check-audit.2). *Three files nobody will ever split* and *thirty-two
waiting on work someone has to do* are different facts about a codebase, and a
single number is actionable as neither.

The fixture also pins what an unstated kind means: the second entry omits `kind`,
still loads, and counts as `deferred` — the reading that keeps `until` meaningful
and claims no constraint the author never asserted (§FS-003-exceptions.2.1).
