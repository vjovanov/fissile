# E2E-008-exception-add: exception add appends a structured registry entry

`fissile exception add` is the supported way to record a justified oversized
file: it appends a structured entry — id, path, matcher, rules, accepted
maximum, retirement condition, and rationale — to the configured registry rather
than making users hand-edit TOML (§FS-005-exception-add).
