# E2E-004-check-json: the JSON surface is one flat record per finding

`fissile check --format json` emits a flat array of finding records — the agent
surface (§GOAL-004-token-thrift.1) whose shape is the published contract
(`schema/check.schema.json`). The record carries the file, measured value,
limit, severity, rule, and message fields of `check` (§FS-004-check-audit.1).
