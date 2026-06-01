# E2E-005-exception-silences-hard: a hard exception accepts the file but the soft warning survives

A hard-registry exception sized at or above the file silences the blocking
finding, so `fissile check` exits zero — but the soft warning still appears so an
agent keeps minimizing accepted debt. This is the registry override
(§GOAL-007-justified-exceptions) and the silenced-hard-keeps-soft rule
(§FS-003-exceptions.3).
