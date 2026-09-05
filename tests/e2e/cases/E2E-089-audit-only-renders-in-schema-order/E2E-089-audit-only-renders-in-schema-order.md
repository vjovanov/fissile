# E2E-089-audit-only-renders-in-schema-order: the order named is not the order printed

`--only coverage,stale` names coverage first and gets stale first, because
sections render in the canonical order `schema/audit.schema.json` publishes
(§FS-004-check-audit.2). A caller cannot rearrange the report by rearranging the
flag, so a diff of two runs over the same tree still shows only the real change
(§GOAL-004-token-thrift.1).

Both sections are computed because they were named: neither `--stale-exceptions`
nor `--rule-coverage` is passed. The registry entry points at a path no file
stands at, so the stale section has a line to print rather than the `none` that
would prove less.
