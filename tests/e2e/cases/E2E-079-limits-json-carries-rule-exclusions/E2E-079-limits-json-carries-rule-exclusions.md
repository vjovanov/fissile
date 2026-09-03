# E2E-079-limits-json-carries-rule-exclusions: machine inventory exposes only non-empty negative scope

The JSON inventory places a non-empty `exclude` array after `include`, preserving
declaration order, and omits it for the byte catch-all whose exclusion list is
empty (§FS-010-limits.4). That omission keeps existing consumers' records stable
when version 1 configurations do not use the optional scope
(§FS-001-config.3.4).
