# E2E-092-audit-only-is-a-text-flag: the selector stops where the JSON contract starts

`--only` exists because the text report is one document with a fixed head, while
the JSON report is already a set of independently addressable keys
(§FS-004-check-audit.2). Bringing the flag to `--format json` would be either a
schema violation or a no-op, and both are refusals worth making out loud.

A violation, because `findings`, `silenced` and `exceptions` are `required` in
`schema/audit.schema.json`: an object with those keys filtered out does not
validate against the contract the flag was passed to satisfy. A no-op, because
accepting the flag and ignoring it leaves the caller unable to tell selection
from a section that had nothing in it — which is the same failure the unknown
name of E2E-091 refuses.

So the combination exits `2` and says which of the two flags does not belong,
and a consumer wanting one key keeps the `jq` that already answers it
(§GOAL-004-token-thrift.1).
