# E2E-035-check-reports-a-dangling-exception: check names an entry whose file is gone

An exact-path exception whose file is not on disk accepts nothing. `check`
reports it in a block of its own, named by the registry that holds it
(§FS-004-check-audit.1.3), so the commit that moved or deleted the file is where
the leftover entry surfaces — rather than in an `audit --stale-exceptions` run
someone has to remember to make (§DF-007-instructions-at-the-error-site.1).

The default `[exceptions].stale = "warn"` reports without failing: a leftover
entry is bookkeeping, not a budget violation, so the run still exits `0`.
