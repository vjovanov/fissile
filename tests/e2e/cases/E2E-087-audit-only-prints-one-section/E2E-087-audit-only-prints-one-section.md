# E2E-087-audit-only-prints-one-section: audit prints the section that was asked for, and no other

Rule coverage is read while tuning config, in an edit-run-edit loop. Every
iteration of that loop reprints a findings block that has not changed and that
the reader is not looking at, and `| tail -N` cannot cut it because the coverage
section's own length varies with the repository.

`--only coverage` is the answer: the named section, and nothing else — no
findings, no silenced lines, no `exceptions:` counts (§FS-004-check-audit.2).
This tree has all three, so each one is a section the flag has to suppress
rather than a section that happened to be empty.

Naming a section is also the request to compute it, so no `--rule-coverage` is
passed here. That flag is what naming the section already says
(§GOAL-004-token-thrift.1).
