# E2E-041-an-excluded-file-is-not-a-dead-entry: out of scope is not gone

The registry accepts `src/vendor/big.rs`, which `[scan].exclude` keeps out of the
inventory. The file is in the working tree, unmoved, so the entry has outlived
nothing and the run is `ok` under `[exceptions].stale = "error"`
(§FS-004-check-audit.1.3).

A full scan compares the registry against the scan scope, and a path the scope
excludes — or that git ignores — is missing from it for a reason that says
nothing about the file. Reporting it would print "the file moved or was deleted"
about a file that did neither, and under `error` that false statement would fail
the build. `audit` still counts the entry (§FS-004-check-audit.2), where the
answer is a report rather than a gate.
