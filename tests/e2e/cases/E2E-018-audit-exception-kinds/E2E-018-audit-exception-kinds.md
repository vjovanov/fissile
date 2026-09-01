# E2E-018-audit-exception-kinds: audit separates accepted-permanently from carrying-debt

`audit` reports both registry-entry and distinct path-expression totals for each
kind (§FS-004-check-audit.2). A soft/hard twin is two entries but one path, so
the fixture proves that *two accepted entries across one structural path* and
*one entry across one deferred path* remain different facts about a codebase.

The fixture also pins what an unstated kind means: the second hard entry omits
`kind`, still loads, and counts as `deferred` — the reading that keeps `until`
meaningful and claims no constraint the author never asserted
(§FS-003-exceptions.2.1).
