# E2E-078-limits-prints-rule-exclusions: text inventory exposes negative rule scope

The text inventory prints a non-empty rule exclusion immediately after that
rule's include list (§FS-010-limits.3). The byte catch-all has no exclusions and
keeps the pre-existing line shape, matching the configuration semantics in
§FS-001-config.3.4.
