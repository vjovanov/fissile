# E2E-066-limits-prints-every-configured-rule: the inventory is the config, not the tree

A reader asking what this repository enforces gets every configured rule, in the
order the document declares them, so the output reads beside the config and
diffs against it (§FS-010-limits.2). This tree holds no `src/`, no `assets/` and
no `notes/`, and all three rules still print: filtering by what matched would
make `limits` a second `audit --rule-coverage` and drop exactly the rule a
reader is most likely to be wrong about.

The line carries the id, the include patterns bracketed, the unit, and the
thresholds the rule declares, in `measure`'s spelling (§FS-010-limits.3). The
byte rule declares only `hard` and the notes rule only `soft`, so each prints
one threshold: a placeholder for the other would state a limit the config does
not set.
